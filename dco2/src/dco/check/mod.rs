//! This module contains the DCO check logic.

use crate::github::{Commit, Config, ConfigAllowRemediationCommits, GitUser};
use anyhow::{bail, Result};
use askama::Template;
use email_address::EmailAddress;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, sync::LazyLock};
use thiserror::Error;

mod filters;
#[cfg(test)]
mod tests;

/// Check input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct CheckInput {
    pub commits: Vec<Commit>,
    pub config: Config,
    pub head_ref: String,
}

/// Check output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Template)]
#[template(path = "output.md")]
pub(crate) struct CheckOutput {
    pub commits: Vec<CommitCheckOutput>,
    pub head_ref: String,
    pub num_commits_with_errors: usize,
}

/// Commit check output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct CommitCheckOutput {
    pub commit: Commit,
    pub errors: Vec<CommitError>,
    pub success_reason: Option<CommitSuccessReason>,
}

impl CommitCheckOutput {
    /// Create a new commit check output.
    fn new(commit: Commit) -> Self {
        Self {
            commit,
            errors: Vec::new(),
            success_reason: None,
        }
    }
}

/// Errors that may occur on a given commit during the check.
#[derive(Error, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum CommitError {
    #[error("invalid author email")]
    InvalidAuthorEmail,
    #[error("invalid committer email")]
    InvalidCommitterEmail,
    #[error("no sign-off matches the author or committer")]
    SignOffMismatch,
    #[error("sign-off not found")]
    SignOffNotFound,
}

/// Reasons why a commit's check succeeded.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum CommitSuccessReason {
    FromBot,
    IsMerge,
    ValidSignOff,
    ValidSignOffInRemediationCommit,
}

impl Display for CommitSuccessReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommitSuccessReason::FromBot => write!(f, "skipped: sign-off not required in bot commit"),
            CommitSuccessReason::IsMerge => write!(f, "skipped: sign-off not required in merge commit"),
            CommitSuccessReason::ValidSignOff => write!(f, "valid sign-off found"),
            CommitSuccessReason::ValidSignOffInRemediationCommit => {
                write!(f, "valid sign-off found in remediation commit")
            }
        }
    }
}

/// Run DCO check.
pub(crate) fn check(input: &CheckInput) -> CheckOutput {
    let mut output = CheckOutput {
        commits: Vec::new(),
        head_ref: input.head_ref.clone(),
        num_commits_with_errors: 0,
    };

    // Get remediations from all commits
    let remediations = get_remediations(&input.config.allow_remediation_commits, &input.commits);

    // Check each commit
    for commit in &input.commits {
        let mut commit_output = CommitCheckOutput::new(commit.clone());

        // Check if we should skip this commit
        let (commit_should_be_skipped, reason) = should_skip_commit(commit);
        if commit_should_be_skipped {
            commit_output.success_reason = reason;
            output.commits.push(commit_output);
            continue;
        }

        // Validate author and committer emails
        let emails_are_valid = match validate_emails(commit) {
            Ok(()) => true,
            Err(errors) => {
                commit_output.errors.extend(errors);
                false
            }
        };

        // Check if sign-off is present
        let signoffs = get_signoffs(commit);
        if signoffs.is_empty() {
            commit_output.errors.push(CommitError::SignOffNotFound);
        }

        // Check if any of the sign-offs matches the author's or committer's email
        if emails_are_valid && !signoffs.is_empty() {
            if signoffs_match(&signoffs, commit) {
                commit_output.success_reason = Some(CommitSuccessReason::ValidSignOff);
            } else {
                commit_output.errors.push(CommitError::SignOffMismatch);
            }
        }

        // Check if the sign-off is present in a remediation commit
        if commit_output.success_reason.is_none() && remediations_match(&remediations, commit) {
            commit_output.errors.clear();
            commit_output.success_reason = Some(CommitSuccessReason::ValidSignOffInRemediationCommit);
        }

        // Track commit
        output.commits.push(commit_output);
    }

    // Update output status
    output.num_commits_with_errors = output.commits.iter().filter(|c| !c.errors.is_empty()).count();

    output
}

/// Check if we should skip this commit.
fn should_skip_commit(commit: &Commit) -> (bool, Option<CommitSuccessReason>) {
    // Skip merge commits
    if commit.is_merge {
        return (true, Some(CommitSuccessReason::IsMerge));
    }

    // Skip bots commits
    if let Some(author) = &commit.author {
        if author.is_bot {
            return (true, Some(CommitSuccessReason::FromBot));
        }
    }

    (false, None)
}

/// Validate author and committer emails.
fn validate_emails(commit: &Commit) -> Result<(), Vec<CommitError>> {
    let mut errors = Vec::new();

    // Committer
    let committer_email = commit.committer.as_ref().map(|c| &c.email);
    if let Some(committer_email) = committer_email {
        if !EmailAddress::is_valid(committer_email) {
            errors.push(CommitError::InvalidCommitterEmail);
        }
    }

    // Author
    let author_email = commit.author.as_ref().map(|a| &a.email);
    if let Some(author_email) = author_email {
        if Some(author_email) != committer_email && !EmailAddress::is_valid(author_email) {
            errors.push(CommitError::InvalidAuthorEmail);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Sign-off line regular expression.
static SIGN_OFF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?mi)^Signed-off-by: (.*) <(.*)>\s*$").expect("expr in SIGN_OFF to be valid")
});

/// Sign-off details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct SignOff {
    name: String,
    email: String,
}

impl SignOff {
    /// Check if the sign-off matches the provided user (if any).
    fn matches_user(&self, user: &Option<GitUser>) -> bool {
        if let Some(user) = user {
            self.name.to_lowercase() == user.name.to_lowercase()
                && self.email.to_lowercase() == user.email.to_lowercase()
        } else {
            false
        }
    }
}

/// Get sign-offs found in the commit message.
fn get_signoffs(commit: &Commit) -> Vec<SignOff> {
    let mut signoffs = Vec::new();

    for (_, [name, email]) in SIGN_OFF.captures_iter(&commit.message).map(|c| c.extract()) {
        signoffs.push(SignOff {
            name: name.to_string(),
            email: email.to_string(),
        });
    }

    signoffs
}

/// Check if any of the sign-offs matches the author's or committer's email.
fn signoffs_match(signoffs: &[SignOff], commit: &Commit) -> bool {
    signoffs
        .iter()
        .any(|signoff| signoff.matches_user(&commit.author) || signoff.matches_user(&commit.committer))
}

/// Individual remediation regular expression.
static INDIVIDUAL_REMEDIATION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?mi)^I, (.*) <(.*)>, hereby add my Signed-off-by to this commit: (.*)\s*$")
        .expect("expr in INDIVIDUAL_REMEDIATION to be valid")
});

/// Third party remediation regular expression.
static THIRD_PARTY_REMEDIATION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?mi)^On behalf of (.*) <(.*)>, I, (.*) <(.*)>, hereby add my Signed-off-by to this commit: (.*)\s*$")
        .expect("expr in THIRD_PARTY_REMEDIATION to be valid")
});

/// Remediation details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Remediation {
    pub declarant: GitUser,
    pub target_sha: String,
}

impl Remediation {
    /// Create a new remediation.
    fn new(
        declarant_name: &str,
        declarant_email: &str,
        representative_name: Option<&str>,
        representative_email: Option<&str>,
        target_sha: &str,
        commit: &Commit,
    ) -> Result<Self> {
        // Prepare declarant and representative
        let declarant = GitUser {
            name: declarant_name.to_string(),
            email: declarant_email.to_string(),
            ..Default::default()
        };
        let representative = 'representative: {
            let Some(name) = representative_name else {
                break 'representative None;
            };
            let Some(email) = representative_email else {
                break 'representative None;
            };
            Some(GitUser {
                name: name.to_string(),
                email: email.to_string(),
                ..Default::default()
            })
        };

        // If the representative is provided, it must match the author or committer
        if let Some(representative) = &representative {
            if !representative.matches(&commit.author) && !representative.matches(&commit.committer) {
                bail!("representative must match the author or committer");
            }
        } else {
            // Otherwise, the declarant must match the author or committer
            if !declarant.matches(&commit.author) && !declarant.matches(&commit.committer) {
                bail!("declarant must match the author or committer");
            }
        }

        // Create remediation and return it
        Ok(Remediation {
            declarant,
            target_sha: target_sha.to_string(),
        })
    }

    /// Check if the remediation matches the provided commit.
    fn matches_commit(&self, commit: &Commit) -> bool {
        if self.target_sha != commit.sha {
            return false;
        }
        self.declarant.matches(&commit.author) || self.declarant.matches(&commit.committer)
    }
}

/// Get remediations found in the list of commits provided.
fn get_remediations(
    allow_remediation_commits: &Option<ConfigAllowRemediationCommits>,
    commits: &[Commit],
) -> Vec<Remediation> {
    let mut remediations = Vec::new();

    // Nothing to do if this feature isn't enabled in the config
    let Some(allow_remediation_commits) = allow_remediation_commits else {
        return remediations;
    };

    // Collect remediations from commits
    for commit in commits {
        // Collect individual remediations if this feature is enabled
        if allow_remediation_commits.individual.unwrap_or(false) {
            let captures = INDIVIDUAL_REMEDIATION.captures_iter(&commit.message).map(|c| c.extract());
            for (_, [declarant_name, declarant_email, target_sha]) in captures {
                if let Ok(remediation) =
                    Remediation::new(declarant_name, declarant_email, None, None, target_sha, commit)
                {
                    remediations.push(remediation);
                }
            }

            // Collect third-party remediations if this feature is enabled
            if allow_remediation_commits.third_party.unwrap_or(false) {
                let captures = THIRD_PARTY_REMEDIATION.captures_iter(&commit.message).map(|c| c.extract());
                for (
                    _,
                    [declarant_name, declarant_email, representative_name, representative_email, target_sha],
                ) in captures
                {
                    if let Ok(remediation) = Remediation::new(
                        declarant_name,
                        declarant_email,
                        Some(representative_name),
                        Some(representative_email),
                        target_sha,
                        commit,
                    ) {
                        remediations.push(remediation);
                    }
                }
            }
        }
    }

    remediations
}

/// Check if any of the remediations matches the provided commit.
fn remediations_match(remediations: &[Remediation], commit: &Commit) -> bool {
    remediations.iter().any(|remediation| remediation.matches_commit(commit))
}

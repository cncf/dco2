//! This module contains the DCO check logic.

use crate::github::{Commit, Config};
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
}

impl Display for CommitSuccessReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommitSuccessReason::FromBot => write!(f, "sign-off not required in bot commit"),
            CommitSuccessReason::IsMerge => write!(f, "sign-off not required in merge commit"),
            CommitSuccessReason::ValidSignOff => write!(f, "valid sign-off found"),
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
        if emails_are_valid && !signoffs.is_empty() && !signoffs_match(&signoffs, commit) {
            commit_output.errors.push(CommitError::SignOffMismatch);
        }

        // Track commit
        if commit_output.errors.is_empty() {
            commit_output.success_reason = Some(CommitSuccessReason::ValidSignOff);
        }
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
    kind: SignOffKind,
}

/// Sign-off kind.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum SignOffKind {
    Explicit,
    IndividualRemediation,
    ThirdPartyRemediation,
}

/// Get sign-offs found in the commit message.
fn get_signoffs(commit: &Commit) -> Vec<SignOff> {
    let mut signoffs = Vec::new();

    for (_, [name, email]) in SIGN_OFF.captures_iter(&commit.message).map(|c| c.extract()) {
        signoffs.push(SignOff {
            name: name.to_string(),
            email: email.to_string(),
            kind: SignOffKind::Explicit,
        });
    }

    signoffs
}

/// Check if any of the sign-offs matches the author's or committer's email.
fn signoffs_match(signoffs: &[SignOff], commit: &Commit) -> bool {
    let signoff_matches_author = |s: &SignOff| {
        if let Some(a) = &commit.author {
            s.name.to_lowercase() == a.name.to_lowercase() && s.email.to_lowercase() == a.email.to_lowercase()
        } else {
            false
        }
    };

    let signoff_matches_committer = |s: &SignOff| {
        if let Some(c) = &commit.committer {
            s.name.to_lowercase() == c.name.to_lowercase() && s.email.to_lowercase() == c.email.to_lowercase()
        } else {
            false
        }
    };

    signoffs.iter().any(|s| signoff_matches_author(s) || signoff_matches_committer(s))
}

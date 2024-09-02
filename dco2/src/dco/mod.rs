//! This module contains the DCO check logic.

use crate::{
    dco,
    github::{
        CheckRun, CheckRunAction, CheckRunConclusion, CheckRunEvent, CheckRunEventAction, CheckRunStatus,
        Commit, DynGHClient, Event, PullRequestEvent, PullRequestEventAction,
    },
};
use anyhow::{Context, Result};
use askama::Template;
use chrono::Utc;
use email_address::EmailAddress;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use thiserror::Error;
use tracing::debug;

#[cfg(test)]
mod tests;

/// Action to override the check result and set it to passed.
const ACTION_OVERRIDE: &str = "override";

/// Check name.
const CHECK_NAME: &str = "DCO";

/// Sign-off line regular expression.
static SIGN_OFF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?mi)^Signed-off-by: (.*) <(.*)>\s*$").expect("expr in SIGN_OFF to be valid")
});

/// Check input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckInput {
    pub commits: Vec<Commit>,
}

/// Check output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Template)]
#[template(path = "output.md")]
pub struct CheckOutput {
    pub commits_with_errors: Vec<CommitCheckOutput>,
    pub total_commits: usize,
}

/// Commit check output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommitCheckOutput {
    pub commit: Commit,
    pub errors: Vec<CommitError>,
}

impl CommitCheckOutput {
    /// Create a new commit check output.
    fn new(commit: Commit) -> Self {
        Self {
            commit,
            errors: Vec::new(),
        }
    }
}

/// Errors that may occur on a given commit during the check.
#[derive(Error, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CommitError {
    #[error("invalid author email")]
    InvalidAuthorEmail,
    #[error("invalid committer email")]
    InvalidCommitterEmail,
    #[error("no sign-off matches the author or committer")]
    SignOffMismatch,
    #[error("sign-off not found")]
    SignOffNotFound,
}

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
}

/// Process the GitHub webhook event provided.
pub async fn process_event(gh_client: DynGHClient, event: &Event) -> Result<()> {
    // Take the appropriate action for the event received
    match event {
        Event::CheckRun(event) => process_check_run_event(gh_client, event).await,
        Event::PullRequest(event) => process_pull_request_event(gh_client, event).await,
    }
}

/// Process check run event.
pub async fn process_check_run_event(gh_client: DynGHClient, event: &CheckRunEvent) -> Result<()> {
    let ctx = event.ctx();
    let started_at = Utc::now();

    // Check if we are interested in the event action
    if event.action != CheckRunEventAction::RequestedAction {
        return Ok(());
    }

    // Override action: create check run with success status
    if event.requested_action.identifier == ACTION_OVERRIDE {
        let check_run = CheckRun {
            actions: vec![],
            completed_at: Utc::now(),
            conclusion: CheckRunConclusion::Success,
            head_sha: event.check_run.head_sha.clone(),
            name: CHECK_NAME.to_string(),
            started_at,
            status: CheckRunStatus::Completed,
            summary: "Check result was manually set to passed.".to_string(),
        };
        gh_client.create_check_run(&ctx, &check_run).await.context("error creating check run")?;
    }

    Ok(())
}

/// Process pull request event.
pub async fn process_pull_request_event(gh_client: DynGHClient, event: &PullRequestEvent) -> Result<()> {
    let ctx = event.ctx();
    let started_at = Utc::now();

    // Check if we are interested in the event action
    if ![
        PullRequestEventAction::Opened,
        PullRequestEventAction::Synchronize,
    ]
    .contains(&event.action)
    {
        return Ok(());
    }

    // Get PR commits
    let commits: Vec<Commit> = gh_client
        .compare_commits(&ctx, &event.pull_request.base.sha, &event.pull_request.head.sha)
        .await
        .context("error getting pull request commits")?;

    // Run DCO check
    let input = dco::CheckInput { commits };
    let output = dco::check(&input);

    // Create check run
    let (conclusion, actions) = if output.commits_with_errors.is_empty() {
        (CheckRunConclusion::Success, vec![])
    } else {
        (
            CheckRunConclusion::ActionRequired,
            vec![CheckRunAction {
                label: "Set DCO to pass".to_string(),
                description: "Override check result setting it to passed".to_string(),
                identifier: ACTION_OVERRIDE.to_string(),
            }],
        )
    };
    let check_run = CheckRun {
        actions,
        completed_at: Utc::now(),
        conclusion,
        head_sha: event.pull_request.head.sha.clone(),
        name: CHECK_NAME.to_string(),
        started_at,
        status: CheckRunStatus::Completed,
        summary: output.render().context("error rendering output template")?,
    };
    gh_client.create_check_run(&ctx, &check_run).await.context("error creating check run")?;

    Ok(())
}

/// Run DCO check.
pub fn check(input: &CheckInput) -> CheckOutput {
    let mut output = CheckOutput {
        commits_with_errors: Vec::new(),
        total_commits: input.commits.len(),
    };

    // Check each commit
    for commit in &input.commits {
        let mut commit_output = CommitCheckOutput::new(commit.clone());

        // Check if we should skip this commit
        let (commit_should_be_skipped, reason) = should_skip_commit(commit);
        if commit_should_be_skipped {
            debug!("commit ({}) skipped: {:?}", commit_output.commit.sha, reason);
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

        // Track commit if it has errors
        debug_processed_commit(&commit_output, &signoffs);
        if !commit_output.errors.is_empty() {
            output.commits_with_errors.push(commit_output);
        }
    }

    output
}

/// Check if we should skip this commit.
fn should_skip_commit(commit: &Commit) -> (bool, Option<String>) {
    // Skip merge commits
    if commit.is_merge {
        return (true, Some("merge commit".to_string()));
    }

    // Skip bots commits
    if let Some(author) = &commit.author {
        if author.is_bot {
            return (true, Some("author is a bot".to_string()));
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

/// Display some information about a processed commit.
fn debug_processed_commit(commit_output: &CommitCheckOutput, signoffs: &[SignOff]) {
    debug!("commit processed: {}", commit_output.commit.sha);
    debug!("errors found: {:?}", commit_output.errors);
    debug!("author: {:?}", commit_output.commit.author);
    debug!("committer: {:?}", commit_output.commit.committer);
    debug!("sign-offs:");
    for signoff in signoffs {
        debug!("sign-off: {:?}", signoff);
    }
}

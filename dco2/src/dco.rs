//! This module contains the DCO check logic.

use crate::{
    dco,
    github::{CheckRun, Commit, DynGHClient, Event, PullRequestEventAction},
};
use anyhow::{Context, Result};
use askama::Template;
use chrono::Utc;
use email_address::EmailAddress;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use thiserror::Error;

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
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Template)]
#[template(path = "output.md")]
pub struct CheckOutput {
    pub check_passed: bool,
    pub commits: Vec<CommitCheckOutput>,
    pub num_commits_with_errors: usize,
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
    #[error("no sign-off matches the author or committer email")]
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
    let ctx = event.ctx();
    let started_at = Utc::now();

    // Check if we are interested in the PR event action
    let Event::PullRequest(event) = event;
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
    let output = dco::check(&input).context("error running dco check")?;

    // Create check run
    let check_run = CheckRun {
        conclusion: (if output.check_passed { "success" } else { "failure" }).to_string(),
        head_sha: event.pull_request.head.sha.clone(),
        name: "DCO".to_string(),
        started_at,
        status: "completed".to_string(),
        summary: output.render().context("error rendering output template")?,
    };
    gh_client.create_check_run(&ctx, &check_run).await.context("error creating check run")?;

    Ok(())
}

/// Run DCO check.
pub fn check(input: &CheckInput) -> Result<CheckOutput> {
    let mut output = CheckOutput::default();

    // Check each commit
    for commit in &input.commits {
        let mut commit_output = CommitCheckOutput::new(commit.clone());

        // Validate author and committer emails
        if let Err(err) = validate_emails(commit) {
            commit_output.errors.push(err);
        }

        // Check if sign-off is present
        let signoffs = get_signoffs(commit);
        if signoffs.is_empty() {
            commit_output.errors.push(CommitError::SignOffNotFound);
        } else {
            // Check if any of the sign-offs matches the author's or committer's email
            if !signoffs_match(&signoffs, commit) {
                commit_output.errors.push(CommitError::SignOffMismatch);
            }
        }

        output.commits.push(commit_output);
    }

    // Update check output status
    output.check_passed = output.commits.iter().all(|c| c.errors.is_empty());
    output.num_commits_with_errors = output.commits.iter().filter(|c| !c.errors.is_empty()).count();

    Ok(output)
}

/// Validate author and committer emails.
fn validate_emails(commit: &Commit) -> Result<(), CommitError> {
    // Author
    if let Some(author) = &commit.author {
        if !EmailAddress::is_valid(&author.email) {
            return Err(CommitError::InvalidAuthorEmail);
        }
    }

    // Committer
    if let Some(committer) = &commit.committer {
        if !EmailAddress::is_valid(&committer.email) {
            return Err(CommitError::InvalidCommitterEmail);
        }
    }

    Ok(())
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
    let author_email = commit.author.as_ref().map(|a| &a.email);
    let committer_email = commit.committer.as_ref().map(|c| &c.email);

    signoffs
        .iter()
        .any(|s| Some(&s.email) == author_email || Some(&s.email) == committer_email)
}

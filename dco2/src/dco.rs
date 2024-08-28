//! This module contains the DCO check logic.

use crate::{
    dco,
    github::{CheckRun, Commit, DynGHClient, Event, PullRequestEventAction},
};
use anyhow::{Context, Result};
use askama::Template;
use chrono::Utc;
use serde::{Deserialize, Serialize};

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

/// DCO check input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckInput {
    pub commits: Vec<Commit>,
}

/// DCO check output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Template)]
#[template(path = "output.md")]
pub struct CheckOutput {
    pub check_passed: bool,
}

/// Run DCO check.
pub fn check(_input: &CheckInput) -> Result<CheckOutput> {
    Ok(CheckOutput { check_passed: true })
}

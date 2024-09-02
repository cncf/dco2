//! This module contains the logic to process GitHub webhook events.

use super::check::{check, CheckInput};
use crate::github::{
    CheckRun, CheckRunAction, CheckRunConclusion, CheckRunEvent, CheckRunEventAction, CheckRunStatus, Commit,
    DynGHClient, Event, PullRequestEvent, PullRequestEventAction,
};
use anyhow::{Context, Result};
use askama::Template;
use chrono::Utc;

/// Check name that will be displayed in GitHub.
const CHECK_NAME: &str = "DCO";

/// Action to override the check result and set it to passed.
const OVERRIDE: &str = "override";

/// Process the GitHub webhook event provided, taking the appropriate action.
pub async fn process_event(gh_client: DynGHClient, event: &Event) -> Result<()> {
    match event {
        Event::CheckRun(event) => process_check_run_event(gh_client, event).await,
        Event::PullRequest(event) => process_pull_request_event(gh_client, event).await,
    }
}

/// Process check run event.
async fn process_check_run_event(gh_client: DynGHClient, event: &CheckRunEvent) -> Result<()> {
    let ctx = event.ctx();
    let started_at = Utc::now();

    // Check if we are interested in the event action
    if event.action != CheckRunEventAction::RequestedAction {
        return Ok(());
    }

    // Override: create check run with success status
    if event.requested_action.identifier == OVERRIDE {
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
async fn process_pull_request_event(gh_client: DynGHClient, event: &PullRequestEvent) -> Result<()> {
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

    // Get pull request commits
    let commits: Vec<Commit> = gh_client
        .compare_commits(&ctx, &event.pull_request.base.sha, &event.pull_request.head.sha)
        .await
        .context("error getting pull request commits")?;

    // Run DCO check
    let input = CheckInput { commits };
    let output = check(&input);

    // Create check run
    let (conclusion, actions) = if output.commits_with_errors.is_empty() {
        (CheckRunConclusion::Success, vec![])
    } else {
        (
            CheckRunConclusion::ActionRequired,
            vec![CheckRunAction {
                label: "Set DCO to pass".to_string(),
                description: "Manually set DCO check result to passed".to_string(),
                identifier: OVERRIDE.to_string(),
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

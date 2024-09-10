//! This module contains the logic to process GitHub webhook events.

use super::check::{check, CheckInput};
use crate::github::{
    CheckRun, CheckRunAction, CheckRunConclusion, CheckRunEvent, CheckRunEventAction, CheckRunStatus, Commit,
    DynGHClient, Event, MergeGroupEvent, MergeGroupEventAction, NewCheckRunInput, PullRequestEvent,
    PullRequestEventAction,
};
use anyhow::{Context, Result};
use askama::Template;
use chrono::Utc;

#[cfg(test)]
mod tests;

/// Name of the check that will be displayed in GitHub.
const CHECK_NAME: &str = "DCO";

/// Title of the check run when the check fails.
const CHECK_FAILED_TITLE: &str = "Check failed";

/// Title of the check run when the check passes.
const CHECK_PASSED_TITLE: &str = "Check passed!";

/// Summary of the check when requested by a merge group.
const MERGE_GROUP_CHECKS_REQUESTED_SUMMARY: &str = "Check result set to passed for the merge group";

/// Identifier of the override action (set check result to passed).
const OVERRIDE_ACTION_IDENTIFIER: &str = "override";

/// Label of the override action.
const OVERRIDE_ACTION_LABEL: &str = "Set DCO to pass";

/// Description of the override action.
const OVERRIDE_ACTION_DESCRIPTION: &str = "Manually set DCO check result to passed";

/// Summary of the override action.
const OVERRIDE_ACTION_SUMMARY: &str = "Check result was manually set to passed";

/// Process the GitHub webhook event provided, taking the appropriate action.
pub async fn process_event(gh_client: DynGHClient, event: &Event) -> Result<()> {
    match event {
        Event::CheckRun(event) => process_check_run_event(gh_client, event).await,
        Event::MergeGroup(event) => process_merge_group_event(gh_client, event).await,
        Event::PullRequest(event) => process_pull_request_event(gh_client, event).await,
    }
}

/// Process check run event.
async fn process_check_run_event(gh_client: DynGHClient, event: &CheckRunEvent) -> Result<()> {
    let started_at = Utc::now();
    let ctx = event.ctx();

    // Check if we are interested in the event action
    if event.action != CheckRunEventAction::RequestedAction {
        return Ok(());
    }

    // Override: create check run with success status
    if let Some(requested_action) = &event.requested_action {
        if requested_action.identifier == OVERRIDE_ACTION_IDENTIFIER {
            let check_run = CheckRun::new(NewCheckRunInput {
                actions: vec![],
                completed_at: Utc::now(),
                conclusion: CheckRunConclusion::Success,
                head_sha: event.check_run.head_sha.clone(),
                name: CHECK_NAME.to_string(),
                started_at,
                status: CheckRunStatus::Completed,
                summary: OVERRIDE_ACTION_SUMMARY.to_string(),
                title: OVERRIDE_ACTION_SUMMARY.to_string(),
            });
            gh_client.create_check_run(&ctx, &check_run).await.context("error creating check run")?;
        }
    }

    Ok(())
}

/// Process merge group event.
async fn process_merge_group_event(gh_client: DynGHClient, event: &MergeGroupEvent) -> Result<()> {
    let started_at = Utc::now();
    let ctx = event.ctx();

    // Create a check run with success status when checks are requested for a
    // merge group. The DCO check must already have passed before the pull
    // request was added to the merge queue, so there is no need to run it again
    if event.action != MergeGroupEventAction::ChecksRequested {
        return Ok(());
    }
    let check_run = CheckRun::new(NewCheckRunInput {
        actions: vec![],
        completed_at: Utc::now(),
        conclusion: CheckRunConclusion::Success,
        head_sha: event.merge_group.head_commit.id.clone(),
        name: CHECK_NAME.to_string(),
        started_at,
        status: CheckRunStatus::Completed,
        summary: MERGE_GROUP_CHECKS_REQUESTED_SUMMARY.to_string(),
        title: MERGE_GROUP_CHECKS_REQUESTED_SUMMARY.to_string(),
    });
    gh_client.create_check_run(&ctx, &check_run).await.context("error creating check run")?;

    Ok(())
}

/// Process pull request event.
async fn process_pull_request_event(gh_client: DynGHClient, event: &PullRequestEvent) -> Result<()> {
    let started_at = Utc::now();
    let ctx = event.ctx();

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

    // Get repository configuration
    let config = gh_client
        .get_config(&ctx)
        .await
        .context("error getting repository configuration")?
        .unwrap_or_default();

    // Create a list of members that are not required to sign-off commits
    let mut members = vec![];
    if !config.members_signoff_is_required() {
        members = collect_members(gh_client.clone(), event, &commits)
            .await
            .context("error collecting members")?
    };

    // Run DCO check
    let input = CheckInput {
        commits,
        config,
        head_ref: event.pull_request.head.ref_.clone(),
        members,
    };
    let output = check(&input);

    // Create check run
    let (conclusion, title, actions) = if output.num_commits_with_errors == 0 {
        (CheckRunConclusion::Success, CHECK_PASSED_TITLE, vec![])
    } else {
        (
            CheckRunConclusion::ActionRequired,
            CHECK_FAILED_TITLE,
            vec![CheckRunAction {
                label: OVERRIDE_ACTION_LABEL.to_string(),
                description: OVERRIDE_ACTION_DESCRIPTION.to_string(),
                identifier: OVERRIDE_ACTION_IDENTIFIER.to_string(),
            }],
        )
    };
    let check_run = CheckRun::new(NewCheckRunInput {
        actions,
        completed_at: Utc::now(),
        conclusion,
        head_sha: event.pull_request.head.sha.clone(),
        name: CHECK_NAME.to_string(),
        started_at,
        status: CheckRunStatus::Completed,
        summary: output.render().context("error rendering output template")?,
        title: title.to_string(),
    });
    gh_client.create_check_run(&ctx, &check_run).await.context("error creating check run")?;

    Ok(())
}

/// Create a list of members that are not required to sign-off commits.
async fn collect_members(
    gh_client: DynGHClient,
    event: &PullRequestEvent,
    commits: &[Commit],
) -> Result<Vec<String>> {
    let mut members = vec![];

    // If the repository belongs to an organization, collect its members
    let ctx = event.ctx();
    if let Some(org) = event.organization.as_ref().map(|o| o.login.as_str()) {
        for commit in commits {
            if commit.verified.unwrap_or(false) {
                // Check if the commit's author is a member of the organization
                if let Some(author_username) = commit.author.as_ref().and_then(|a| a.login.clone()) {
                    if !members.contains(&author_username)
                        && gh_client
                            .is_organization_member(&ctx, org, &author_username)
                            .await
                            .context("error checking organization membership")?
                    {
                        members.push(author_username)
                    }
                }
            }
        }
    } else {
        // Otherwise, the only member will be the repository owner
        members.push(event.repository.owner.login.to_string());
    }

    Ok(members)
}

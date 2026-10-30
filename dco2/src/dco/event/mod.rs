//! This module contains the logic to process GitHub webhook events.

use std::{
    collections::{BTreeSet, HashSet},
    time::Duration,
};

use anyhow::{Context, Result};
use askama::Template;
use chrono::Utc;
use tokio::time::timeout;
use tracing::{debug, info, warn};

use crate::{
    dco::check::{CheckInput, check},
    github::{
        CheckRun, CheckRunAction, CheckRunConclusion, CheckRunEvent, CheckRunEventAction, CheckRunStatus,
        Commit, Config, Ctx, DynGHClient, Event, LabelsPermissionError, MergeGroupEvent,
        MergeGroupEventAction, NewCheckRunInput, PullRequestEvent, PullRequestEventAction, PullRequestLabel,
        is_rate_limit_error,
    },
};

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

/// Maximum time the synchronization of the label of a pull request can take,
/// so that it does not delay the check run (e.g. when requests are retried).
const SYNC_LABEL_TIMEOUT: Duration = Duration::from_secs(3);

/// Input used to synchronize the label of a pull request. When the event
/// includes the labels of the pull request, they are used to skip the
/// synchronization if no change is needed.
struct SyncLabelInput<'a> {
    head_sha: &'a str,
    labels: Option<&'a [PullRequestLabel]>,
    pr_number: i64,
    should_be_labeled: bool,
}

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
    if let Some(requested_action) = &event.requested_action
        && requested_action.identifier == OVERRIDE_ACTION_IDENTIFIER
    {
        // Ignore override requests when the repository disabled this action
        let config = gh_client
            .get_config(&ctx)
            .await
            .context("error getting repository configuration")?
            .unwrap_or_default();
        if !config.override_action_is_allowed() {
            return Ok(());
        }

        let check_run = CheckRun::new(NewCheckRunInput {
            actions: vec![],
            completed_at: Utc::now(),
            conclusion: CheckRunConclusion::Success,
            external_id: event.check_run.external_id.clone(),
            head_sha: event.check_run.head_sha.clone(),
            name: CHECK_NAME.to_string(),
            started_at,
            status: CheckRunStatus::Completed,
            summary: OVERRIDE_ACTION_SUMMARY.to_string(),
            title: OVERRIDE_ACTION_SUMMARY.to_string(),
        });
        gh_client.create_check_run(&ctx, &check_run).await.context("error creating check run")?;

        // Remove the label from the pull requests associated with the check
        // run: the one it was created for (external id) and those reported
        // by GitHub that target this repository
        let pr_numbers: BTreeSet<i64> = event
            .check_run
            .external_id
            .iter()
            .filter_map(|external_id| external_id.parse().ok())
            .chain(
                event
                    .check_run
                    .pull_requests
                    .iter()
                    .filter(|pr| pr.base.repo.id == event.repository.id)
                    .map(|pr| pr.number),
            )
            .collect();
        for pr_number in pr_numbers {
            let input = SyncLabelInput {
                head_sha: &event.check_run.head_sha,
                labels: None,
                pr_number,
                should_be_labeled: false,
            };
            sync_label(&gh_client, &ctx, &config, &input).await;
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
        external_id: None,
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
    let check_failed = output.num_commits_with_errors > 0;

    // Prepare check run
    let (conclusion, title, actions) = if !check_failed {
        (CheckRunConclusion::Success, CHECK_PASSED_TITLE, vec![])
    } else {
        let actions = if output.config.override_action_is_allowed() {
            vec![CheckRunAction {
                label: OVERRIDE_ACTION_LABEL.to_string(),
                description: OVERRIDE_ACTION_DESCRIPTION.to_string(),
                identifier: OVERRIDE_ACTION_IDENTIFIER.to_string(),
            }]
        } else {
            vec![]
        };

        (CheckRunConclusion::ActionRequired, CHECK_FAILED_TITLE, actions)
    };
    let check_run = CheckRun::new(NewCheckRunInput {
        actions,
        completed_at: Utc::now(),
        conclusion,
        external_id: Some(event.number.to_string()),
        head_sha: event.pull_request.head.sha.clone(),
        name: CHECK_NAME.to_string(),
        started_at,
        status: CheckRunStatus::Completed,
        summary: output.render().context("error rendering output template")?,
        title: title.to_string(),
    });

    // Add or remove the label depending on the check result. This is done
    // before creating the check run so that the override action can't be
    // used until the label has been synchronized
    let input = SyncLabelInput {
        head_sha: &event.pull_request.head.sha,
        labels: Some(&event.pull_request.labels),
        pr_number: event.number,
        should_be_labeled: check_failed,
    };
    sync_label(&gh_client, &ctx, &output.config, &input).await;

    // Create check run
    gh_client.create_check_run(&ctx, &check_run).await.context("error creating check run")?;

    Ok(())
}

/// Create a list of members that are not required to sign-off commits.
async fn collect_members(
    gh_client: DynGHClient,
    event: &PullRequestEvent,
    commits: &[Commit],
) -> Result<Vec<String>> {
    let mut checked_authors = HashSet::new();
    let mut members = vec![];

    // If the repository belongs to an organization, collect its members
    let ctx = event.ctx();
    if let Some(org) = event.organization.as_ref().map(|o| o.login.as_str()) {
        for commit in commits {
            if !commit.verified.unwrap_or(false) {
                continue;
            }
            if let Some(author) = commit.author.as_ref()
                && !author.is_bot
                && let Some(author_username) = author.login.as_ref()
                && checked_authors.insert(author_username.to_string())
            {
                let is_member = gh_client
                    .is_organization_member(&ctx, org, author_username)
                    .await
                    .context("error checking organization membership")?;
                if is_member {
                    members.push(author_username.to_string());
                }
            }
        }
    } else {
        // Otherwise, the only member will be the repository owner
        members.push(event.repository.owner.login.to_string());
    }

    Ok(members)
}

/// Find the label provided in the list of labels of a pull request, returning
/// the name of the applied label (label names are case-insensitive).
fn find_label<'a>(labels: impl IntoIterator<Item = &'a str>, label: &str) -> Option<&'a str> {
    let label_lowercase = label.to_lowercase();
    labels.into_iter().find(|name| name.to_lowercase() == label_lowercase)
}

/// Check if the error provided was caused by the app lacking the permissions
/// required to manage labels.
fn is_labels_permission_error(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| cause.is::<LabelsPermissionError>())
}

/// Synchronize the configured label of a pull request with the result of the
/// DCO check. This is a best-effort operation that must not prevent the check
/// run from being created, so errors are logged instead of returned.
async fn sync_label(gh_client: &DynGHClient, ctx: &Ctx, config: &Config, input: &SyncLabelInput<'_>) {
    // Give up when the synchronization takes too long
    let Ok(result) = timeout(SYNC_LABEL_TIMEOUT, try_sync_label(gh_client, ctx, config, input)).await else {
        warn!(
            owner = %ctx.owner,
            repo = %ctx.repo,
            pr_number = input.pr_number,
            "label sync timed out"
        );
        return;
    };

    // Log the error found, if any
    if let Err(err) = result {
        // Missing permissions are expected until the installation owner
        // approves them (and until access tokens issued before are renewed)
        if is_labels_permission_error(&err) {
            info!(
                owner = %ctx.owner,
                repo = %ctx.repo,
                pr_number = input.pr_number,
                "permissions to manage labels not granted, skipping label sync"
            );
            return;
        }

        // Rate limits are logged separately to make them easier to spot
        if is_rate_limit_error(&err) {
            warn!(
                ?err,
                owner = %ctx.owner,
                repo = %ctx.repo,
                pr_number = input.pr_number,
                "rate limited by GitHub, skipping label sync"
            );
            return;
        }

        warn!(
            ?err,
            owner = %ctx.owner,
            repo = %ctx.repo,
            pr_number = input.pr_number,
            "error syncing label"
        );
    }
}

/// Synchronize the configured label of a pull request with the result of the
/// DCO check, returning any error found.
async fn try_sync_label(
    gh_client: &DynGHClient,
    ctx: &Ctx,
    config: &Config,
    input: &SyncLabelInput<'_>,
) -> Result<()> {
    // Skip when labeling is disabled or the configured label is not valid
    let Some(label) = config.label() else {
        if config.label.is_some() {
            warn!(
                owner = %ctx.owner,
                repo = %ctx.repo,
                "invalid label configured, skipping label sync"
            );
        }
        return Ok(());
    };

    // Skip when the labels included in the event show that no change is
    // needed (the current labels are checked again before changing them)
    if let Some(labels) = input.labels
        && find_label(labels.iter().map(|l| l.name.as_str()), label).is_some() == input.should_be_labeled
    {
        return Ok(());
    }

    // Skip when the permissions required to manage labels are not granted
    if !gh_client.can_manage_labels(ctx).await.context("error checking labels permissions")? {
        info!(
            owner = %ctx.owner,
            repo = %ctx.repo,
            pr_number = input.pr_number,
            "permissions to manage labels not granted, skipping label sync"
        );
        return Ok(());
    }

    // Get the current state of the pull request, skipping it when closed or
    // when its head has moved (a newer event will handle the new commits)
    let pr = gh_client
        .get_pull_request(ctx, input.pr_number)
        .await
        .context("error getting pull request")?;
    if !pr.is_open || pr.head_sha != input.head_sha {
        debug!(
            owner = %ctx.owner,
            repo = %ctx.repo,
            pr_number = input.pr_number,
            "pull request closed or head moved, skipping label sync"
        );
        return Ok(());
    }

    // Add or remove the label when needed (the name of the applied label is
    // used when removing it, as it may differ in case from the configured one)
    let applied_label = find_label(pr.labels.iter().map(String::as_str), label);
    match (input.should_be_labeled, applied_label) {
        (true, None) => {
            gh_client.add_label(ctx, input.pr_number, label).await.context("error adding label")?;
            info!(owner = %ctx.owner, repo = %ctx.repo, pr_number = input.pr_number, label, "label added");
        }
        (false, Some(applied_label)) => {
            gh_client
                .remove_label(ctx, input.pr_number, applied_label)
                .await
                .context("error removing label")?;
            info!(
                owner = %ctx.owner,
                repo = %ctx.repo,
                pr_number = input.pr_number,
                label = applied_label,
                "label removed"
            );
        }
        _ => {}
    }

    Ok(())
}

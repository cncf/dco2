use std::{future, sync::Arc};

use anyhow::{Ok, anyhow};
use indoc::indoc;
use mockall::{Sequence, predicate::eq};

use crate::{
    dco::{
        event::{
            CHECK_FAILED_TITLE, CHECK_NAME, CHECK_PASSED_TITLE, MERGE_GROUP_CHECKS_REQUESTED_SUMMARY,
            OVERRIDE_ACTION_DESCRIPTION, OVERRIDE_ACTION_IDENTIFIER, OVERRIDE_ACTION_LABEL,
            OVERRIDE_ACTION_SUMMARY, is_labels_permission_error,
        },
        process_event,
    },
    github::{
        CheckRunAction, CheckRunConclusion, CheckRunEvent, CheckRunEventAction, CheckRunEventCheckRun,
        CheckRunEventPullRequest, CheckRunEventPullRequestBase, CheckRunEventPullRequestRepository,
        CheckRunStatus, Commit, Config, ConfigRequire, Ctx, Event, Installation, LabelsPermissionError,
        MergeGroupEvent, MergeGroupEventAction, MergeGroupEventMergeGroup, MergeGroupHeadCommit,
        MockGHClient, Organization, PullRequest, PullRequestBase, PullRequestEvent, PullRequestEventAction,
        PullRequestHead, PullRequestLabel, PullRequestState, Repository, RepositoryOwner, RequestedAction,
        User,
    },
};

const LABEL: &str = "needs-dco";

#[tokio::test]
async fn check_run_event_other_action() {
    let event = CheckRunEvent {
        action: CheckRunEventAction::Other,
        check_run: CheckRunEventCheckRun {
            external_id: None,
            head_sha: "head_sha".to_string(),
            pull_requests: vec![],
        },
        installation: Installation { id: 1 },
        repository: Repository {
            id: 1,
            name: "repo".to_string(),
            owner: RepositoryOwner {
                login: "owner".to_string(),
            },
        },
        requested_action: None,
    };

    let gh_client = MockGHClient::new();

    process_event(Arc::new(gh_client), &Event::CheckRun(event)).await.unwrap();
}

#[tokio::test]
async fn check_run_event_requested_action_unknown_identifier() {
    let event = CheckRunEvent {
        action: CheckRunEventAction::RequestedAction,
        check_run: CheckRunEventCheckRun {
            external_id: None,
            head_sha: "head_sha".to_string(),
            pull_requests: vec![],
        },
        installation: Installation { id: 1 },
        repository: Repository {
            id: 1,
            name: "repo".to_string(),
            owner: RepositoryOwner {
                login: "owner".to_string(),
            },
        },
        requested_action: Some(RequestedAction {
            identifier: "unknown".to_string(),
        }),
    };

    let gh_client = MockGHClient::new();

    process_event(Arc::new(gh_client), &Event::CheckRun(event)).await.unwrap();
}

#[tokio::test]
#[should_panic(expected = "error creating check run")]
async fn check_run_event_requested_action_override_error_creating_check_run() {
    let event = CheckRunEvent {
        action: CheckRunEventAction::RequestedAction,
        check_run: CheckRunEventCheckRun {
            external_id: None,
            head_sha: "head_sha".to_string(),
            pull_requests: vec![],
        },
        installation: Installation { id: 1 },
        repository: Repository {
            id: 1,
            name: "repo".to_string(),
            owner: RepositoryOwner {
                login: "owner".to_string(),
            },
        },
        requested_action: Some(RequestedAction {
            identifier: OVERRIDE_ACTION_IDENTIFIER.to_string(),
        }),
    };

    let mut gh_client = MockGHClient::new();
    gh_client
        .expect_get_config()
        .with(eq(event.ctx()))
        .times(1)
        .returning(|_| Box::pin(future::ready(Ok(Some(Config::default())))));
    let expected_ctx = event.ctx();
    gh_client
        .expect_create_check_run()
        .withf(move |ctx, check_run| {
            *ctx == expected_ctx
                && check_run.actions().is_empty()
                && check_run.completed_at() >= check_run.started_at()
                && check_run.conclusion() == &CheckRunConclusion::Success
                && check_run.head_sha() == "head_sha"
                && check_run.name() == CHECK_NAME
                && check_run.status() == &CheckRunStatus::Completed
                && check_run.summary() == OVERRIDE_ACTION_SUMMARY
                && check_run.title() == OVERRIDE_ACTION_SUMMARY
        })
        .times(1)
        .returning(|_, _| Box::pin(future::ready(Err(anyhow!("test error")))));

    process_event(Arc::new(gh_client), &Event::CheckRun(event)).await.unwrap();
}

#[tokio::test]
async fn check_run_event_requested_action_override_ignored_when_disabled() {
    let event = CheckRunEvent {
        action: CheckRunEventAction::RequestedAction,
        check_run: CheckRunEventCheckRun {
            external_id: None,
            head_sha: "head_sha".to_string(),
            pull_requests: vec![],
        },
        installation: Installation { id: 1 },
        repository: Repository {
            id: 1,
            name: "repo".to_string(),
            owner: RepositoryOwner {
                login: "owner".to_string(),
            },
        },
        requested_action: Some(RequestedAction {
            identifier: OVERRIDE_ACTION_IDENTIFIER.to_string(),
        }),
    };

    let mut gh_client = MockGHClient::new();
    gh_client.expect_get_config().with(eq(event.ctx())).times(1).returning(|_| {
        Box::pin(future::ready(Ok(Some(Config {
            allow_override_action: Some(false),
            ..Default::default()
        }))))
    });

    process_event(Arc::new(gh_client), &Event::CheckRun(event)).await.unwrap();
}

#[tokio::test]
async fn check_run_event_requested_action_override_label_disabled() {
    let event = override_event(Some("42"), &[]);
    let mut gh_client = MockGHClient::new();
    expect_override_check(&mut gh_client, &event, Config::default());
    gh_client.expect_can_manage_labels().never();
    gh_client.expect_get_pull_request().never();
    gh_client.expect_remove_label().never();

    process_event(Arc::new(gh_client), &Event::CheckRun(event)).await.unwrap();
}

#[tokio::test]
async fn check_run_event_requested_action_override_label_error_removing_label() {
    let event = override_event(Some("42"), &[]);
    let mut gh_client = MockGHClient::new();
    expect_override_check(&mut gh_client, &event, config_with_label());
    expect_can_manage_labels(&mut gh_client, &event.ctx(), true);
    expect_get_pull_request(
        &mut gh_client,
        &event.ctx(),
        42,
        pr_state("head_sha", true, &[LABEL]),
    );
    gh_client
        .expect_remove_label()
        .with(eq(event.ctx()), eq(42), eq(LABEL))
        .times(1)
        .returning(|_, _, _| Box::pin(future::ready(Err(anyhow!("test error")))));

    process_event(Arc::new(gh_client), &Event::CheckRun(event)).await.unwrap();
}

#[tokio::test]
async fn check_run_event_requested_action_override_label_ignores_other_repositories_pull_requests() {
    let event = override_event(Some(""), &[(43, 99)]);
    let mut gh_client = MockGHClient::new();
    expect_override_check(&mut gh_client, &event, config_with_label());
    gh_client.expect_can_manage_labels().never();
    gh_client.expect_get_pull_request().never();
    gh_client.expect_remove_label().never();

    process_event(Arc::new(gh_client), &Event::CheckRun(event)).await.unwrap();
}

#[tokio::test]
async fn check_run_event_requested_action_override_label_permission_not_granted() {
    let event = override_event(Some("42"), &[]);
    let mut gh_client = MockGHClient::new();
    expect_override_check(&mut gh_client, &event, config_with_label());
    expect_can_manage_labels(&mut gh_client, &event.ctx(), false);
    gh_client.expect_get_pull_request().never();
    gh_client.expect_remove_label().never();

    process_event(Arc::new(gh_client), &Event::CheckRun(event)).await.unwrap();
}

#[tokio::test]
async fn check_run_event_requested_action_override_label_removed_from_associated_pull_requests() {
    let event = override_event(Some("42"), &[(42, 20), (43, 20)]);
    let mut gh_client = MockGHClient::new();
    expect_override_check(&mut gh_client, &event, config_with_label());
    gh_client
        .expect_can_manage_labels()
        .with(eq(event.ctx()))
        .times(2)
        .returning(|_| Box::pin(future::ready(Ok(true))));
    expect_get_pull_request(
        &mut gh_client,
        &event.ctx(),
        42,
        pr_state("head_sha", true, &[LABEL]),
    );
    expect_get_pull_request(
        &mut gh_client,
        &event.ctx(),
        43,
        pr_state("head_sha", true, &[LABEL]),
    );
    expect_remove_label(&mut gh_client, &event.ctx(), 42, LABEL);
    expect_remove_label(&mut gh_client, &event.ctx(), 43, LABEL);

    process_event(Arc::new(gh_client), &Event::CheckRun(event)).await.unwrap();
}

#[tokio::test]
async fn check_run_event_requested_action_override_label_removed_using_external_id() {
    let event = override_event(Some("42"), &[]);
    let mut gh_client = MockGHClient::new();
    expect_override_check(&mut gh_client, &event, config_with_label());
    expect_can_manage_labels(&mut gh_client, &event.ctx(), true);
    expect_get_pull_request(
        &mut gh_client,
        &event.ctx(),
        42,
        pr_state("head_sha", true, &[LABEL]),
    );
    expect_remove_label(&mut gh_client, &event.ctx(), 42, LABEL);

    process_event(Arc::new(gh_client), &Event::CheckRun(event)).await.unwrap();
}

#[tokio::test]
async fn check_run_event_requested_action_override_label_removed_using_pull_requests() {
    let event = override_event(Some(""), &[(43, 20)]);
    let mut gh_client = MockGHClient::new();
    expect_override_check(&mut gh_client, &event, config_with_label());
    expect_can_manage_labels(&mut gh_client, &event.ctx(), true);
    expect_get_pull_request(
        &mut gh_client,
        &event.ctx(),
        43,
        pr_state("head_sha", true, &[LABEL]),
    );
    expect_remove_label(&mut gh_client, &event.ctx(), 43, LABEL);

    process_event(Arc::new(gh_client), &Event::CheckRun(event)).await.unwrap();
}

#[tokio::test]
async fn check_run_event_requested_action_override_label_skipped_when_head_moved() {
    let event = override_event(Some("42"), &[]);
    let mut gh_client = MockGHClient::new();
    expect_override_check(&mut gh_client, &event, config_with_label());
    expect_can_manage_labels(&mut gh_client, &event.ctx(), true);
    expect_get_pull_request(
        &mut gh_client,
        &event.ctx(),
        42,
        pr_state("new_head_sha", true, &[LABEL]),
    );
    gh_client.expect_remove_label().never();

    process_event(Arc::new(gh_client), &Event::CheckRun(event)).await.unwrap();
}

#[tokio::test]
async fn check_run_event_requested_action_override_label_skipped_without_pull_requests() {
    let event = override_event(Some(""), &[]);
    let mut gh_client = MockGHClient::new();
    expect_override_check(&mut gh_client, &event, config_with_label());
    gh_client.expect_can_manage_labels().never();
    gh_client.expect_get_pull_request().never();
    gh_client.expect_remove_label().never();

    process_event(Arc::new(gh_client), &Event::CheckRun(event)).await.unwrap();
}

#[tokio::test]
async fn check_run_event_requested_action_override_success() {
    let event = CheckRunEvent {
        action: CheckRunEventAction::RequestedAction,
        check_run: CheckRunEventCheckRun {
            external_id: None,
            head_sha: "head_sha".to_string(),
            pull_requests: vec![],
        },
        installation: Installation { id: 1 },
        repository: Repository {
            id: 1,
            name: "repo".to_string(),
            owner: RepositoryOwner {
                login: "owner".to_string(),
            },
        },
        requested_action: Some(RequestedAction {
            identifier: OVERRIDE_ACTION_IDENTIFIER.to_string(),
        }),
    };

    let mut gh_client = MockGHClient::new();
    gh_client
        .expect_get_config()
        .with(eq(event.ctx()))
        .times(1)
        .returning(|_| Box::pin(future::ready(Ok(Some(Config::default())))));
    let expected_ctx = event.ctx();
    gh_client
        .expect_create_check_run()
        .withf(move |ctx, check_run| {
            *ctx == expected_ctx
                && check_run.actions().is_empty()
                && check_run.completed_at() >= check_run.started_at()
                && check_run.conclusion() == &CheckRunConclusion::Success
                && check_run.head_sha() == "head_sha"
                && check_run.name() == CHECK_NAME
                && check_run.status() == &CheckRunStatus::Completed
                && check_run.summary() == OVERRIDE_ACTION_SUMMARY
                && check_run.title() == OVERRIDE_ACTION_SUMMARY
        })
        .times(1)
        .returning(|_, _| Box::pin(future::ready(Ok(()))));

    process_event(Arc::new(gh_client), &Event::CheckRun(event)).await.unwrap();
}

#[test]
fn is_labels_permission_error_other_error() {
    let err = anyhow!("test error").context("error adding label");
    assert!(!is_labels_permission_error(&err));
}

#[test]
fn is_labels_permission_error_permission_error_with_context() {
    let err = anyhow::Error::new(LabelsPermissionError).context("error adding label");
    assert!(is_labels_permission_error(&err));
}

#[tokio::test]
async fn merge_group_other_action() {
    let event = MergeGroupEvent {
        action: MergeGroupEventAction::Other,
        merge_group: MergeGroupEventMergeGroup {
            head_commit: MergeGroupHeadCommit {
                id: "head_sha".to_string(),
            },
        },
        installation: Installation { id: 1 },
        repository: Repository {
            id: 1,
            name: "repo".to_string(),
            owner: RepositoryOwner {
                login: "owner".to_string(),
            },
        },
    };

    let gh_client = MockGHClient::new();

    process_event(Arc::new(gh_client), &Event::MergeGroup(event)).await.unwrap();
}

#[tokio::test]
#[should_panic(expected = "error creating check run")]
async fn merge_group_checks_requested_error_creating_check_run() {
    let event = MergeGroupEvent {
        action: MergeGroupEventAction::ChecksRequested,
        merge_group: MergeGroupEventMergeGroup {
            head_commit: MergeGroupHeadCommit {
                id: "head_sha".to_string(),
            },
        },
        installation: Installation { id: 1 },
        repository: Repository {
            id: 1,
            name: "repo".to_string(),
            owner: RepositoryOwner {
                login: "owner".to_string(),
            },
        },
    };

    let mut gh_client = MockGHClient::new();
    let expected_ctx = event.ctx();
    gh_client
        .expect_create_check_run()
        .withf(move |ctx, check_run| {
            *ctx == expected_ctx
                && check_run.actions().is_empty()
                && check_run.completed_at() >= check_run.started_at()
                && check_run.conclusion() == &CheckRunConclusion::Success
                && check_run.head_sha() == "head_sha"
                && check_run.name() == CHECK_NAME
                && check_run.status() == &CheckRunStatus::Completed
                && check_run.summary() == MERGE_GROUP_CHECKS_REQUESTED_SUMMARY
                && check_run.title() == MERGE_GROUP_CHECKS_REQUESTED_SUMMARY
                && check_run.external_id().is_none()
        })
        .times(1)
        .returning(|_, _| Box::pin(future::ready(Err(anyhow!("test error")))));

    process_event(Arc::new(gh_client), &Event::MergeGroup(event)).await.unwrap();
}

#[tokio::test]
async fn merge_group_checks_requested_success() {
    let event = MergeGroupEvent {
        action: MergeGroupEventAction::ChecksRequested,
        merge_group: MergeGroupEventMergeGroup {
            head_commit: MergeGroupHeadCommit {
                id: "head_sha".to_string(),
            },
        },
        installation: Installation { id: 1 },
        repository: Repository {
            id: 1,
            name: "repo".to_string(),
            owner: RepositoryOwner {
                login: "owner".to_string(),
            },
        },
    };

    let mut gh_client = MockGHClient::new();
    let expected_ctx = event.ctx();
    gh_client
        .expect_create_check_run()
        .withf(move |ctx, check_run| {
            *ctx == expected_ctx
                && check_run.actions().is_empty()
                && check_run.completed_at() >= check_run.started_at()
                && check_run.conclusion() == &CheckRunConclusion::Success
                && check_run.head_sha() == "head_sha"
                && check_run.name() == CHECK_NAME
                && check_run.status() == &CheckRunStatus::Completed
                && check_run.summary() == MERGE_GROUP_CHECKS_REQUESTED_SUMMARY
                && check_run.title() == MERGE_GROUP_CHECKS_REQUESTED_SUMMARY
                && check_run.external_id().is_none()
        })
        .times(1)
        .returning(|_, _| Box::pin(future::ready(Ok(()))));

    process_event(Arc::new(gh_client), &Event::MergeGroup(event)).await.unwrap();
}

#[tokio::test]
async fn pull_request_event_label_added_before_check_run_when_check_fails() {
    let event = pr_opened_event(&[]);
    let mut gh_client = MockGHClient::new();
    let commit = commit_with_signoff(false);
    gh_client
        .expect_compare_commits()
        .with(eq(event.ctx()), eq("base_sha"), eq("head_sha"))
        .times(1)
        .returning(move |_, _, _| Box::pin(future::ready(Ok(vec![commit.clone()]))));
    gh_client
        .expect_get_config()
        .with(eq(event.ctx()))
        .times(1)
        .returning(|_| Box::pin(future::ready(Ok(Some(config_with_label())))));

    let mut seq = Sequence::new();
    expect_can_manage_labels(&mut gh_client, &event.ctx(), true);
    expect_get_pull_request(
        &mut gh_client,
        &event.ctx(),
        42,
        pr_state("head_sha", true, &["other"]),
    );
    gh_client
        .expect_add_label()
        .with(eq(event.ctx()), eq(42), eq(LABEL))
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_, _, _| Box::pin(future::ready(Ok(()))));
    let expected_ctx = event.ctx();
    gh_client
        .expect_create_check_run()
        .withf(move |ctx, check_run| {
            *ctx == expected_ctx
                && check_run.conclusion() == &CheckRunConclusion::ActionRequired
                && check_run.external_id() == Some("42")
        })
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_, _| Box::pin(future::ready(Ok(()))));

    process_event(Arc::new(gh_client), &Event::PullRequest(event)).await.unwrap();
}

#[tokio::test]
async fn pull_request_event_label_disabled() {
    let event = pr_opened_event(&[]);
    let mut gh_client = MockGHClient::new();
    expect_pr_check(&mut gh_client, &event, Config::default(), false);
    gh_client.expect_can_manage_labels().never();
    gh_client.expect_get_pull_request().never();
    gh_client.expect_add_label().never();

    process_event(Arc::new(gh_client), &Event::PullRequest(event)).await.unwrap();
}

#[tokio::test]
async fn pull_request_event_label_error_adding_label() {
    let event = pr_opened_event(&[]);
    let mut gh_client = MockGHClient::new();
    expect_pr_check(&mut gh_client, &event, config_with_label(), false);
    expect_can_manage_labels(&mut gh_client, &event.ctx(), true);
    expect_get_pull_request(&mut gh_client, &event.ctx(), 42, pr_state("head_sha", true, &[]));
    gh_client
        .expect_add_label()
        .with(eq(event.ctx()), eq(42), eq(LABEL))
        .times(1)
        .returning(|_, _, _| Box::pin(future::ready(Err(anyhow!("test error")))));

    process_event(Arc::new(gh_client), &Event::PullRequest(event)).await.unwrap();
}

#[tokio::test]
async fn pull_request_event_label_error_checking_permissions() {
    let event = pr_opened_event(&[]);
    let mut gh_client = MockGHClient::new();
    expect_pr_check(&mut gh_client, &event, config_with_label(), false);
    gh_client
        .expect_can_manage_labels()
        .with(eq(event.ctx()))
        .times(1)
        .returning(|_| Box::pin(future::ready(Err(anyhow!("test error")))));
    gh_client.expect_get_pull_request().never();
    gh_client.expect_add_label().never();

    process_event(Arc::new(gh_client), &Event::PullRequest(event)).await.unwrap();
}

#[tokio::test]
async fn pull_request_event_label_error_getting_pull_request() {
    let event = pr_opened_event(&[]);
    let mut gh_client = MockGHClient::new();
    expect_pr_check(&mut gh_client, &event, config_with_label(), false);
    expect_can_manage_labels(&mut gh_client, &event.ctx(), true);
    gh_client
        .expect_get_pull_request()
        .with(eq(event.ctx()), eq(42))
        .times(1)
        .returning(|_, _| Box::pin(future::ready(Err(anyhow!("test error")))));
    gh_client.expect_add_label().never();

    process_event(Arc::new(gh_client), &Event::PullRequest(event)).await.unwrap();
}

#[tokio::test]
async fn pull_request_event_label_invalid() {
    let event = pr_opened_event(&[]);
    let mut gh_client = MockGHClient::new();
    let config = Config {
        label: Some("  ".to_string()),
        ..Default::default()
    };
    expect_pr_check(&mut gh_client, &event, config, false);
    gh_client.expect_can_manage_labels().never();
    gh_client.expect_get_pull_request().never();
    gh_client.expect_add_label().never();

    process_event(Arc::new(gh_client), &Event::PullRequest(event)).await.unwrap();
}

#[tokio::test]
async fn pull_request_event_label_kept_when_check_fails_and_label_applied() {
    let event = pr_opened_event(&["Needs-DCO"]);
    let mut gh_client = MockGHClient::new();
    expect_pr_check(&mut gh_client, &event, config_with_label(), false);
    gh_client.expect_can_manage_labels().never();
    gh_client.expect_get_pull_request().never();
    gh_client.expect_add_label().never();
    gh_client.expect_remove_label().never();

    process_event(Arc::new(gh_client), &Event::PullRequest(event)).await.unwrap();
}

#[tokio::test]
async fn pull_request_event_label_not_added_when_check_passes() {
    let event = pr_opened_event(&["other"]);
    let mut gh_client = MockGHClient::new();
    expect_pr_check(&mut gh_client, &event, config_with_label(), true);
    gh_client.expect_can_manage_labels().never();
    gh_client.expect_get_pull_request().never();
    gh_client.expect_add_label().never();
    gh_client.expect_remove_label().never();

    process_event(Arc::new(gh_client), &Event::PullRequest(event)).await.unwrap();
}

#[tokio::test]
async fn pull_request_event_label_not_added_when_label_applied_after_event() {
    let event = pr_opened_event(&[]);
    let mut gh_client = MockGHClient::new();
    expect_pr_check(&mut gh_client, &event, config_with_label(), false);
    expect_can_manage_labels(&mut gh_client, &event.ctx(), true);
    expect_get_pull_request(
        &mut gh_client,
        &event.ctx(),
        42,
        pr_state("head_sha", true, &["Needs-DCO"]),
    );
    gh_client.expect_add_label().never();
    gh_client.expect_remove_label().never();

    process_event(Arc::new(gh_client), &Event::PullRequest(event)).await.unwrap();
}

#[tokio::test]
async fn pull_request_event_label_not_removed_when_label_removed_after_event() {
    let event = pr_opened_event(&["Needs-DCO"]);
    let mut gh_client = MockGHClient::new();
    expect_pr_check(&mut gh_client, &event, config_with_label(), true);
    expect_can_manage_labels(&mut gh_client, &event.ctx(), true);
    expect_get_pull_request(&mut gh_client, &event.ctx(), 42, pr_state("head_sha", true, &[]));
    gh_client.expect_add_label().never();
    gh_client.expect_remove_label().never();

    process_event(Arc::new(gh_client), &Event::PullRequest(event)).await.unwrap();
}

#[tokio::test]
async fn pull_request_event_label_permission_not_granted() {
    let event = pr_opened_event(&[]);
    let mut gh_client = MockGHClient::new();
    expect_pr_check(&mut gh_client, &event, config_with_label(), false);
    expect_can_manage_labels(&mut gh_client, &event.ctx(), false);
    gh_client.expect_get_pull_request().never();
    gh_client.expect_add_label().never();

    process_event(Arc::new(gh_client), &Event::PullRequest(event)).await.unwrap();
}

#[tokio::test]
async fn pull_request_event_label_removed_when_check_passes() {
    let event = pr_opened_event(&["Needs-DCO"]);
    let mut gh_client = MockGHClient::new();
    expect_pr_check(&mut gh_client, &event, config_with_label(), true);
    expect_can_manage_labels(&mut gh_client, &event.ctx(), true);
    expect_get_pull_request(
        &mut gh_client,
        &event.ctx(),
        42,
        pr_state("head_sha", true, &["Needs-DCO"]),
    );
    expect_remove_label(&mut gh_client, &event.ctx(), 42, "Needs-DCO");

    process_event(Arc::new(gh_client), &Event::PullRequest(event)).await.unwrap();
}

#[tokio::test]
async fn pull_request_event_label_skipped_when_head_moved() {
    let event = pr_opened_event(&[]);
    let mut gh_client = MockGHClient::new();
    expect_pr_check(&mut gh_client, &event, config_with_label(), false);
    expect_can_manage_labels(&mut gh_client, &event.ctx(), true);
    expect_get_pull_request(
        &mut gh_client,
        &event.ctx(),
        42,
        pr_state("new_head_sha", true, &[]),
    );
    gh_client.expect_add_label().never();

    process_event(Arc::new(gh_client), &Event::PullRequest(event)).await.unwrap();
}

#[tokio::test]
async fn pull_request_event_label_skipped_when_pull_request_closed() {
    let event = pr_opened_event(&[]);
    let mut gh_client = MockGHClient::new();
    expect_pr_check(&mut gh_client, &event, config_with_label(), false);
    expect_can_manage_labels(&mut gh_client, &event.ctx(), true);
    expect_get_pull_request(&mut gh_client, &event.ctx(), 42, pr_state("head_sha", false, &[]));
    gh_client.expect_add_label().never();

    process_event(Arc::new(gh_client), &Event::PullRequest(event)).await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn pull_request_event_label_sync_timed_out() {
    let event = pr_opened_event(&[]);
    let mut gh_client = MockGHClient::new();
    expect_pr_check(&mut gh_client, &event, config_with_label(), false);
    expect_can_manage_labels(&mut gh_client, &event.ctx(), true);
    gh_client
        .expect_get_pull_request()
        .with(eq(event.ctx()), eq(42))
        .times(1)
        .returning(|_, _| Box::pin(future::pending()));
    gh_client.expect_add_label().never();

    process_event(Arc::new(gh_client), &Event::PullRequest(event)).await.unwrap();
}

#[tokio::test]
async fn pull_request_event_other_action() {
    let event = PullRequestEvent {
        action: PullRequestEventAction::Other,
        installation: Installation { id: 1 },
        number: 1,
        organization: None,
        pull_request: PullRequest {
            base: PullRequestBase {
                ref_: "base_ref".to_string(),
                sha: "base_sha".to_string(),
            },
            head: PullRequestHead {
                ref_: "head_ref".to_string(),
                sha: "head_sha".to_string(),
            },
            html_url: "url".to_string(),
            labels: vec![],
        },
        repository: Repository {
            id: 1,
            name: "repo".to_string(),
            owner: RepositoryOwner {
                login: "owner".to_string(),
            },
        },
    };

    let gh_client = MockGHClient::new();

    process_event(Arc::new(gh_client), &Event::PullRequest(event)).await.unwrap();
}

#[tokio::test]
#[should_panic(expected = "error getting pull request commits")]
async fn pull_request_event_opened_action_error_getting_pr_commits() {
    let event = PullRequestEvent {
        action: PullRequestEventAction::Opened,
        installation: Installation { id: 1 },
        number: 1,
        organization: None,
        pull_request: PullRequest {
            base: PullRequestBase {
                ref_: "base_ref".to_string(),
                sha: "base_sha".to_string(),
            },
            head: PullRequestHead {
                ref_: "head_ref".to_string(),
                sha: "head_sha".to_string(),
            },
            html_url: "url".to_string(),
            labels: vec![],
        },
        repository: Repository {
            id: 1,
            name: "repo".to_string(),
            owner: RepositoryOwner {
                login: "owner".to_string(),
            },
        },
    };

    let mut gh_client = MockGHClient::new();
    gh_client
        .expect_compare_commits()
        .with(eq(event.ctx()), eq("base_sha"), eq("head_sha"))
        .times(1)
        .returning(|_, _, _| Box::pin(future::ready(Err(anyhow!("test error")))));

    process_event(Arc::new(gh_client), &Event::PullRequest(event)).await.unwrap();
}

#[tokio::test]
#[should_panic(expected = "error getting repository configuration")]
async fn pull_request_event_opened_action_error_getting_repository_configuration() {
    let event = PullRequestEvent {
        action: PullRequestEventAction::Opened,
        installation: Installation { id: 1 },
        number: 1,
        organization: None,
        pull_request: PullRequest {
            base: PullRequestBase {
                ref_: "base_ref".to_string(),
                sha: "base_sha".to_string(),
            },
            head: PullRequestHead {
                ref_: "head_ref".to_string(),
                sha: "head_sha".to_string(),
            },
            html_url: "url".to_string(),
            labels: vec![],
        },
        repository: Repository {
            id: 1,
            name: "repo".to_string(),
            owner: RepositoryOwner {
                login: "owner".to_string(),
            },
        },
    };

    let mut gh_client = MockGHClient::new();
    gh_client
        .expect_compare_commits()
        .with(eq(event.ctx()), eq("base_sha"), eq("head_sha"))
        .times(1)
        .returning(|_, _, _| {
            Box::pin(future::ready(Ok(vec![Commit {
                author: Some(User {
                    name: "user1".to_string(),
                    email: "user1@email.test".to_string(),
                    ..Default::default()
                }),
                committer: Some(User {
                    name: "user1".to_string(),
                    email: "user1@email.test".to_string(),
                    ..Default::default()
                }),
                message: indoc! {r"
                    Test commit message

                    Signed-off-by: user1 <user1@email.test>
                "}
                .to_string(),
                ..Default::default()
            }])))
        });
    gh_client
        .expect_get_config()
        .with(eq(event.ctx()))
        .times(1)
        .returning(|_| Box::pin(future::ready(Err(anyhow!("test error")))));

    process_event(Arc::new(gh_client), &Event::PullRequest(event)).await.unwrap();
}

#[tokio::test]
#[should_panic(expected = "error checking organization membership")]
async fn pull_request_event_opened_action_error_checking_user_organization_membership() {
    let event = PullRequestEvent {
        action: PullRequestEventAction::Opened,
        installation: Installation { id: 1 },
        number: 1,
        organization: Some(Organization {
            login: "org".to_string(),
        }),
        pull_request: PullRequest {
            base: PullRequestBase {
                ref_: "base_ref".to_string(),
                sha: "base_sha".to_string(),
            },
            head: PullRequestHead {
                ref_: "head_ref".to_string(),
                sha: "head_sha".to_string(),
            },
            html_url: "url".to_string(),
            labels: vec![],
        },
        repository: Repository {
            id: 1,
            name: "repo".to_string(),
            owner: RepositoryOwner {
                login: "owner".to_string(),
            },
        },
    };

    let mut gh_client = MockGHClient::new();
    gh_client
        .expect_compare_commits()
        .with(eq(event.ctx()), eq("base_sha"), eq("head_sha"))
        .times(1)
        .returning(|_, _, _| {
            Box::pin(future::ready(Ok(vec![Commit {
                author: Some(User {
                    name: "user1".to_string(),
                    email: "user1@email.test".to_string(),
                    login: Some("user1".to_string()),
                    ..Default::default()
                }),
                committer: Some(User {
                    name: "user1".to_string(),
                    email: "user1@email.test".to_string(),
                    login: Some("user1".to_string()),
                    ..Default::default()
                }),
                message: "Test commit message".to_string(),
                verified: Some(true),
                ..Default::default()
            }])))
        });
    gh_client.expect_get_config().with(eq(event.ctx())).times(1).returning(|_| {
        Box::pin(future::ready(Ok(Some(Config {
            require: Some(ConfigRequire { members: Some(false) }),
            ..Default::default()
        }))))
    });
    gh_client
        .expect_is_organization_member()
        .with(eq(event.ctx()), eq("org"), eq("user1"))
        .times(1)
        .returning(|_, _, _| Box::pin(future::ready(Err(anyhow!("test error")))));

    process_event(Arc::new(gh_client), &Event::PullRequest(event)).await.unwrap();
}

#[tokio::test]
#[should_panic(expected = "error creating check run")]
async fn pull_request_event_opened_action_error_creating_check_run() {
    let event = PullRequestEvent {
        action: PullRequestEventAction::Opened,
        installation: Installation { id: 1 },
        number: 1,
        organization: None,
        pull_request: PullRequest {
            base: PullRequestBase {
                ref_: "base_ref".to_string(),
                sha: "base_sha".to_string(),
            },
            head: PullRequestHead {
                ref_: "head_ref".to_string(),
                sha: "head_sha".to_string(),
            },
            html_url: "url".to_string(),
            labels: vec![],
        },
        repository: Repository {
            id: 1,
            name: "repo".to_string(),
            owner: RepositoryOwner {
                login: "owner".to_string(),
            },
        },
    };

    let mut gh_client = MockGHClient::new();
    gh_client
        .expect_compare_commits()
        .with(eq(event.ctx()), eq("base_sha"), eq("head_sha"))
        .times(1)
        .returning(|_, _, _| {
            Box::pin(future::ready(Ok(vec![Commit {
                author: Some(User {
                    name: "user1".to_string(),
                    email: "user1@email.test".to_string(),
                    ..Default::default()
                }),
                committer: Some(User {
                    name: "user1".to_string(),
                    email: "user1@email.test".to_string(),
                    ..Default::default()
                }),
                message: indoc! {r"
                    Test commit message

                    Signed-off-by: user1 <user1@email.test>
                "}
                .to_string(),
                ..Default::default()
            }])))
        });
    gh_client
        .expect_get_config()
        .with(eq(event.ctx()))
        .times(1)
        .returning(|_| Box::pin(future::ready(Ok(None))));
    let expected_ctx = event.ctx();
    gh_client
        .expect_create_check_run()
        .withf(move |ctx, check_run| {
            *ctx == expected_ctx
                && check_run.actions().is_empty()
                && check_run.completed_at() >= check_run.started_at()
                && check_run.conclusion() == &CheckRunConclusion::Success
                && check_run.head_sha() == "head_sha"
                && check_run.name() == CHECK_NAME
                && check_run.status() == &CheckRunStatus::Completed
                && check_run.title() == CHECK_PASSED_TITLE
        })
        .times(1)
        .returning(|_, _| Box::pin(future::ready(Err(anyhow!("test error")))));

    process_event(Arc::new(gh_client), &Event::PullRequest(event)).await.unwrap();
}

#[tokio::test]
async fn pull_request_event_opened_action_success_check_passed() {
    let event = PullRequestEvent {
        action: PullRequestEventAction::Opened,
        installation: Installation { id: 1 },
        number: 1,
        organization: None,
        pull_request: PullRequest {
            base: PullRequestBase {
                ref_: "base_ref".to_string(),
                sha: "base_sha".to_string(),
            },
            head: PullRequestHead {
                ref_: "head_ref".to_string(),
                sha: "head_sha".to_string(),
            },
            html_url: "url".to_string(),
            labels: vec![],
        },
        repository: Repository {
            id: 1,
            name: "repo".to_string(),
            owner: RepositoryOwner {
                login: "owner".to_string(),
            },
        },
    };

    let mut gh_client = MockGHClient::new();
    gh_client
        .expect_compare_commits()
        .with(eq(event.ctx()), eq("base_sha"), eq("head_sha"))
        .times(1)
        .returning(|_, _, _| {
            Box::pin(future::ready(Ok(vec![Commit {
                author: Some(User {
                    name: "user1".to_string(),
                    email: "user1@email.test".to_string(),
                    ..Default::default()
                }),
                committer: Some(User {
                    name: "user1".to_string(),
                    email: "user1@email.test".to_string(),
                    ..Default::default()
                }),
                message: indoc! {r"
                    Test commit message

                    Signed-off-by: user1 <user1@email.test>
                "}
                .to_string(),
                ..Default::default()
            }])))
        });
    gh_client
        .expect_get_config()
        .with(eq(event.ctx()))
        .times(1)
        .returning(|_| Box::pin(future::ready(Ok(Some(Config::default())))));
    let expected_ctx = event.ctx();
    gh_client
        .expect_create_check_run()
        .withf(move |ctx, check_run| {
            *ctx == expected_ctx
                && check_run.actions().is_empty()
                && check_run.completed_at() >= check_run.started_at()
                && check_run.conclusion() == &CheckRunConclusion::Success
                && check_run.head_sha() == "head_sha"
                && check_run.name() == CHECK_NAME
                && check_run.status() == &CheckRunStatus::Completed
                && check_run.title() == CHECK_PASSED_TITLE
        })
        .times(1)
        .returning(|_, _| Box::pin(future::ready(Ok(()))));

    process_event(Arc::new(gh_client), &Event::PullRequest(event)).await.unwrap();
}

#[tokio::test]
async fn pull_request_event_opened_action_success_check_passed_author_is_member() {
    let event = PullRequestEvent {
        action: PullRequestEventAction::Opened,
        installation: Installation { id: 1 },
        number: 1,
        organization: Some(Organization {
            login: "org".to_string(),
        }),
        pull_request: PullRequest {
            base: PullRequestBase {
                ref_: "base_ref".to_string(),
                sha: "base_sha".to_string(),
            },
            head: PullRequestHead {
                ref_: "head_ref".to_string(),
                sha: "head_sha".to_string(),
            },
            html_url: "url".to_string(),
            labels: vec![],
        },
        repository: Repository {
            id: 1,
            name: "repo".to_string(),
            owner: RepositoryOwner {
                login: "owner".to_string(),
            },
        },
    };

    let mut gh_client = MockGHClient::new();
    gh_client
        .expect_compare_commits()
        .with(eq(event.ctx()), eq("base_sha"), eq("head_sha"))
        .times(1)
        .returning(|_, _, _| {
            Box::pin(future::ready(Ok(vec![Commit {
                author: Some(User {
                    name: "user1".to_string(),
                    email: "user1@email.test".to_string(),
                    login: Some("user1".to_string()),
                    ..Default::default()
                }),
                committer: Some(User {
                    name: "user1".to_string(),
                    email: "user1@email.test".to_string(),
                    login: Some("user1".to_string()),
                    ..Default::default()
                }),
                message: "Test commit message".to_string(),
                verified: Some(true),
                ..Default::default()
            }])))
        });
    gh_client.expect_get_config().with(eq(event.ctx())).times(1).returning(|_| {
        Box::pin(future::ready(Ok(Some(Config {
            require: Some(ConfigRequire { members: Some(false) }),
            ..Default::default()
        }))))
    });
    gh_client
        .expect_is_organization_member()
        .with(eq(event.ctx()), eq("org"), eq("user1"))
        .times(1)
        .returning(|_, _, _| Box::pin(future::ready(Ok(true))));
    let expected_ctx = event.ctx();
    gh_client
        .expect_create_check_run()
        .withf(move |ctx, check_run| {
            *ctx == expected_ctx
                && check_run.actions().is_empty()
                && check_run.completed_at() >= check_run.started_at()
                && check_run.conclusion() == &CheckRunConclusion::Success
                && check_run.head_sha() == "head_sha"
                && check_run.name() == CHECK_NAME
                && check_run.status() == &CheckRunStatus::Completed
                && check_run.title() == CHECK_PASSED_TITLE
        })
        .times(1)
        .returning(|_, _| Box::pin(future::ready(Ok(()))));

    process_event(Arc::new(gh_client), &Event::PullRequest(event)).await.unwrap();
}

#[tokio::test]
async fn pull_request_event_opened_action_success_check_failed_author_is_not_member() {
    let event = PullRequestEvent {
        action: PullRequestEventAction::Opened,
        installation: Installation { id: 1 },
        number: 1,
        organization: Some(Organization {
            login: "org".to_string(),
        }),
        pull_request: PullRequest {
            base: PullRequestBase {
                ref_: "base_ref".to_string(),
                sha: "base_sha".to_string(),
            },
            head: PullRequestHead {
                ref_: "head_ref".to_string(),
                sha: "head_sha".to_string(),
            },
            html_url: "url".to_string(),
            labels: vec![],
        },
        repository: Repository {
            id: 1,
            name: "repo".to_string(),
            owner: RepositoryOwner {
                login: "owner".to_string(),
            },
        },
    };

    let mut gh_client = MockGHClient::new();
    gh_client
        .expect_compare_commits()
        .with(eq(event.ctx()), eq("base_sha"), eq("head_sha"))
        .times(1)
        .returning(|_, _, _| {
            Box::pin(future::ready(Ok(vec![Commit {
                author: Some(User {
                    name: "user1".to_string(),
                    email: "user1@email.test".to_string(),
                    login: Some("user1".to_string()),
                    ..Default::default()
                }),
                committer: Some(User {
                    name: "user1".to_string(),
                    email: "user1@email.test".to_string(),
                    login: Some("user1".to_string()),
                    ..Default::default()
                }),
                message: "Test commit message".to_string(),
                verified: Some(true),
                ..Default::default()
            }])))
        });
    gh_client.expect_get_config().with(eq(event.ctx())).times(1).returning(|_| {
        Box::pin(future::ready(Ok(Some(Config {
            require: Some(ConfigRequire { members: Some(false) }),
            ..Default::default()
        }))))
    });
    gh_client
        .expect_is_organization_member()
        .with(eq(event.ctx()), eq("org"), eq("user1"))
        .times(1)
        .returning(|_, _, _| Box::pin(future::ready(Ok(false))));
    let expected_ctx = event.ctx();
    gh_client
        .expect_create_check_run()
        .withf(move |ctx, check_run| {
            *ctx == expected_ctx
                && check_run.actions()
                    == vec![CheckRunAction {
                        label: OVERRIDE_ACTION_LABEL.to_string(),
                        description: OVERRIDE_ACTION_DESCRIPTION.to_string(),
                        identifier: OVERRIDE_ACTION_IDENTIFIER.to_string(),
                    }]
                && check_run.completed_at() >= check_run.started_at()
                && check_run.conclusion() == &CheckRunConclusion::ActionRequired
                && check_run.head_sha() == "head_sha"
                && check_run.name() == CHECK_NAME
                && check_run.status() == &CheckRunStatus::Completed
                && check_run.title() == CHECK_FAILED_TITLE
        })
        .times(1)
        .returning(|_, _| Box::pin(future::ready(Ok(()))));

    process_event(Arc::new(gh_client), &Event::PullRequest(event)).await.unwrap();
}

#[tokio::test]
async fn pull_request_event_opened_action_success_check_passed_author_is_bot_membership_check_skipped() {
    let event = PullRequestEvent {
        action: PullRequestEventAction::Opened,
        installation: Installation { id: 1 },
        number: 1,
        organization: Some(Organization {
            login: "org".to_string(),
        }),
        pull_request: PullRequest {
            base: PullRequestBase {
                ref_: "base_ref".to_string(),
                sha: "base_sha".to_string(),
            },
            head: PullRequestHead {
                ref_: "head_ref".to_string(),
                sha: "head_sha".to_string(),
            },
            html_url: "url".to_string(),
            labels: vec![],
        },
        repository: Repository {
            id: 1,
            name: "repo".to_string(),
            owner: RepositoryOwner {
                login: "owner".to_string(),
            },
        },
    };

    let mut gh_client = MockGHClient::new();
    gh_client
        .expect_compare_commits()
        .with(eq(event.ctx()), eq("base_sha"), eq("head_sha"))
        .times(1)
        .returning(|_, _, _| {
            Box::pin(future::ready(Ok(vec![Commit {
                author: Some(User {
                    name: "bot".to_string(),
                    email: "bot@email.test".to_string(),
                    is_bot: true,
                    login: Some("bot".to_string()),
                }),
                committer: Some(User {
                    name: "bot".to_string(),
                    email: "bot@email.test".to_string(),
                    is_bot: true,
                    login: Some("bot".to_string()),
                }),
                message: "Test commit message".to_string(),
                verified: Some(true),
                ..Default::default()
            }])))
        });
    gh_client.expect_get_config().with(eq(event.ctx())).times(1).returning(|_| {
        Box::pin(future::ready(Ok(Some(Config {
            require: Some(ConfigRequire { members: Some(false) }),
            ..Default::default()
        }))))
    });
    gh_client.expect_is_organization_member().times(0);
    let expected_ctx = event.ctx();
    gh_client
        .expect_create_check_run()
        .withf(move |ctx, check_run| {
            *ctx == expected_ctx
                && check_run.actions().is_empty()
                && check_run.completed_at() >= check_run.started_at()
                && check_run.conclusion() == &CheckRunConclusion::Success
                && check_run.head_sha() == "head_sha"
                && check_run.name() == CHECK_NAME
                && check_run.status() == &CheckRunStatus::Completed
                && check_run.title() == CHECK_PASSED_TITLE
        })
        .times(1)
        .returning(|_, _| Box::pin(future::ready(Ok(()))));

    process_event(Arc::new(gh_client), &Event::PullRequest(event)).await.unwrap();
}

#[tokio::test]
async fn pull_request_event_opened_action_success_check_passed_author_membership_checked_once() {
    let event = PullRequestEvent {
        action: PullRequestEventAction::Opened,
        installation: Installation { id: 1 },
        number: 1,
        organization: Some(Organization {
            login: "org".to_string(),
        }),
        pull_request: PullRequest {
            base: PullRequestBase {
                ref_: "base_ref".to_string(),
                sha: "base_sha".to_string(),
            },
            head: PullRequestHead {
                ref_: "head_ref".to_string(),
                sha: "head_sha".to_string(),
            },
            html_url: "url".to_string(),
            labels: vec![],
        },
        repository: Repository {
            id: 1,
            name: "repo".to_string(),
            owner: RepositoryOwner {
                login: "owner".to_string(),
            },
        },
    };

    let mut gh_client = MockGHClient::new();
    gh_client
        .expect_compare_commits()
        .with(eq(event.ctx()), eq("base_sha"), eq("head_sha"))
        .times(1)
        .returning(|_, _, _| {
            Box::pin(future::ready(Ok(vec![
                Commit {
                    author: Some(User {
                        name: "user1".to_string(),
                        email: "user1@email.test".to_string(),
                        login: Some("user1".to_string()),
                        ..Default::default()
                    }),
                    committer: Some(User {
                        name: "user1".to_string(),
                        email: "user1@email.test".to_string(),
                        login: Some("user1".to_string()),
                        ..Default::default()
                    }),
                    message: "Test commit message".to_string(),
                    verified: Some(true),
                    ..Default::default()
                },
                Commit {
                    author: Some(User {
                        name: "user1".to_string(),
                        email: "user1@email.test".to_string(),
                        login: Some("user1".to_string()),
                        ..Default::default()
                    }),
                    committer: Some(User {
                        name: "user1".to_string(),
                        email: "user1@email.test".to_string(),
                        login: Some("user1".to_string()),
                        ..Default::default()
                    }),
                    message: "Test commit message".to_string(),
                    verified: Some(true),
                    ..Default::default()
                },
            ])))
        });
    gh_client.expect_get_config().with(eq(event.ctx())).times(1).returning(|_| {
        Box::pin(future::ready(Ok(Some(Config {
            require: Some(ConfigRequire { members: Some(false) }),
            ..Default::default()
        }))))
    });
    gh_client
        .expect_is_organization_member()
        .with(eq(event.ctx()), eq("org"), eq("user1"))
        .times(1)
        .returning(|_, _, _| Box::pin(future::ready(Ok(true))));
    let expected_ctx = event.ctx();
    gh_client
        .expect_create_check_run()
        .withf(move |ctx, check_run| {
            *ctx == expected_ctx
                && check_run.actions().is_empty()
                && check_run.completed_at() >= check_run.started_at()
                && check_run.conclusion() == &CheckRunConclusion::Success
                && check_run.head_sha() == "head_sha"
                && check_run.name() == CHECK_NAME
                && check_run.status() == &CheckRunStatus::Completed
                && check_run.title() == CHECK_PASSED_TITLE
        })
        .times(1)
        .returning(|_, _| Box::pin(future::ready(Ok(()))));

    process_event(Arc::new(gh_client), &Event::PullRequest(event)).await.unwrap();
}

#[tokio::test]
async fn pull_request_event_opened_action_success_check_failed() {
    let event = PullRequestEvent {
        action: PullRequestEventAction::Opened,
        installation: Installation { id: 1 },
        number: 1,
        organization: None,
        pull_request: PullRequest {
            base: PullRequestBase {
                ref_: "base_ref".to_string(),
                sha: "base_sha".to_string(),
            },
            head: PullRequestHead {
                ref_: "head_ref".to_string(),
                sha: "head_sha".to_string(),
            },
            html_url: "url".to_string(),
            labels: vec![],
        },
        repository: Repository {
            id: 1,
            name: "repo".to_string(),
            owner: RepositoryOwner {
                login: "owner".to_string(),
            },
        },
    };

    let mut gh_client = MockGHClient::new();
    gh_client
        .expect_compare_commits()
        .with(eq(event.ctx()), eq("base_sha"), eq("head_sha"))
        .times(1)
        .returning(|_, _, _| {
            Box::pin(future::ready(Ok(vec![Commit {
                author: Some(User {
                    name: "user1".to_string(),
                    email: "user1@email.test".to_string(),
                    ..Default::default()
                }),
                committer: Some(User {
                    name: "user1".to_string(),
                    email: "user1@email.test".to_string(),
                    ..Default::default()
                }),
                message: indoc! {r"
                    Test commit message

                    Signed-off-by: userx <userx@email.test>
                "}
                .to_string(),
                ..Default::default()
            }])))
        });
    gh_client
        .expect_get_config()
        .with(eq(event.ctx()))
        .times(1)
        .returning(|_| Box::pin(future::ready(Ok(Some(Config::default())))));
    let expected_ctx = event.ctx();
    gh_client
        .expect_create_check_run()
        .withf(move |ctx, check_run| {
            *ctx == expected_ctx
                && check_run.actions()
                    == vec![CheckRunAction {
                        label: OVERRIDE_ACTION_LABEL.to_string(),
                        description: OVERRIDE_ACTION_DESCRIPTION.to_string(),
                        identifier: OVERRIDE_ACTION_IDENTIFIER.to_string(),
                    }]
                && check_run.completed_at() >= check_run.started_at()
                && check_run.conclusion() == &CheckRunConclusion::ActionRequired
                && check_run.head_sha() == "head_sha"
                && check_run.name() == CHECK_NAME
                && check_run.status() == &CheckRunStatus::Completed
                && check_run.title() == CHECK_FAILED_TITLE
        })
        .times(1)
        .returning(|_, _| Box::pin(future::ready(Ok(()))));

    process_event(Arc::new(gh_client), &Event::PullRequest(event)).await.unwrap();
}

#[tokio::test]
async fn pull_request_event_opened_action_success_check_failed_override_action_disabled() {
    let event = PullRequestEvent {
        action: PullRequestEventAction::Opened,
        installation: Installation { id: 1 },
        number: 1,
        organization: None,
        pull_request: PullRequest {
            base: PullRequestBase {
                ref_: "base_ref".to_string(),
                sha: "base_sha".to_string(),
            },
            head: PullRequestHead {
                ref_: "head_ref".to_string(),
                sha: "head_sha".to_string(),
            },
            html_url: "url".to_string(),
            labels: vec![],
        },
        repository: Repository {
            id: 1,
            name: "repo".to_string(),
            owner: RepositoryOwner {
                login: "owner".to_string(),
            },
        },
    };

    let mut gh_client = MockGHClient::new();
    gh_client
        .expect_compare_commits()
        .with(eq(event.ctx()), eq("base_sha"), eq("head_sha"))
        .times(1)
        .returning(|_, _, _| {
            Box::pin(future::ready(Ok(vec![Commit {
                author: Some(User {
                    name: "user1".to_string(),
                    email: "user1@email.test".to_string(),
                    ..Default::default()
                }),
                committer: Some(User {
                    name: "user1".to_string(),
                    email: "user1@email.test".to_string(),
                    ..Default::default()
                }),
                message: indoc! {r"
                    Test commit message

                    Signed-off-by: userx <userx@email.test>
                "}
                .to_string(),
                ..Default::default()
            }])))
        });
    gh_client.expect_get_config().with(eq(event.ctx())).times(1).returning(|_| {
        Box::pin(future::ready(Ok(Some(Config {
            allow_override_action: Some(false),
            ..Default::default()
        }))))
    });
    let expected_ctx = event.ctx();
    gh_client
        .expect_create_check_run()
        .withf(move |ctx, check_run| {
            *ctx == expected_ctx
                && check_run.actions().is_empty()
                && check_run.completed_at() >= check_run.started_at()
                && check_run.conclusion() == &CheckRunConclusion::ActionRequired
                && check_run.head_sha() == "head_sha"
                && check_run.name() == CHECK_NAME
                && check_run.status() == &CheckRunStatus::Completed
                && check_run.title() == CHECK_FAILED_TITLE
        })
        .times(1)
        .returning(|_, _| Box::pin(future::ready(Ok(()))));

    process_event(Arc::new(gh_client), &Event::PullRequest(event)).await.unwrap();
}

fn commit_with_signoff(is_valid: bool) -> Commit {
    let signoff = if is_valid {
        "user1 <user1@email.test>"
    } else {
        "userx <userx@email.test>"
    };
    Commit {
        author: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        committer: Some(User {
            name: "user1".to_string(),
            email: "user1@email.test".to_string(),
            ..Default::default()
        }),
        message: format!("Test commit message\n\nSigned-off-by: {signoff}\n"),
        ..Default::default()
    }
}

fn config_with_label() -> Config {
    Config {
        label: Some(LABEL.to_string()),
        ..Default::default()
    }
}

fn expect_can_manage_labels(gh_client: &mut MockGHClient, ctx: &Ctx, can_manage_labels: bool) {
    gh_client
        .expect_can_manage_labels()
        .with(eq(ctx.clone()))
        .times(1)
        .returning(move |_| Box::pin(future::ready(Ok(can_manage_labels))));
}

fn expect_get_pull_request(gh_client: &mut MockGHClient, ctx: &Ctx, number: i64, pr: PullRequestState) {
    gh_client
        .expect_get_pull_request()
        .with(eq(ctx.clone()), eq(number))
        .times(1)
        .returning(move |_, _| Box::pin(future::ready(Ok(pr.clone()))));
}

fn expect_override_check(gh_client: &mut MockGHClient, event: &CheckRunEvent, config: Config) {
    gh_client
        .expect_get_config()
        .with(eq(event.ctx()))
        .times(1)
        .returning(move |_| Box::pin(future::ready(Ok(Some(config.clone())))));
    let expected_ctx = event.ctx();
    let expected_external_id = event.check_run.external_id.clone();
    gh_client
        .expect_create_check_run()
        .withf(move |ctx, check_run| {
            *ctx == expected_ctx
                && check_run.conclusion() == &CheckRunConclusion::Success
                && check_run.external_id() == expected_external_id.as_deref()
        })
        .times(1)
        .returning(|_, _| Box::pin(future::ready(Ok(()))));
}

fn expect_pr_check(gh_client: &mut MockGHClient, event: &PullRequestEvent, config: Config, passes: bool) {
    let commit = commit_with_signoff(passes);
    gh_client
        .expect_compare_commits()
        .with(eq(event.ctx()), eq("base_sha"), eq("head_sha"))
        .times(1)
        .returning(move |_, _, _| Box::pin(future::ready(Ok(vec![commit.clone()]))));
    gh_client
        .expect_get_config()
        .with(eq(event.ctx()))
        .times(1)
        .returning(move |_| Box::pin(future::ready(Ok(Some(config.clone())))));
    let expected_conclusion = if passes {
        CheckRunConclusion::Success
    } else {
        CheckRunConclusion::ActionRequired
    };
    let expected_ctx = event.ctx();
    let expected_external_id = event.number.to_string();
    gh_client
        .expect_create_check_run()
        .withf(move |ctx, check_run| {
            *ctx == expected_ctx
                && check_run.conclusion() == &expected_conclusion
                && check_run.external_id() == Some(expected_external_id.as_str())
        })
        .times(1)
        .returning(|_, _| Box::pin(future::ready(Ok(()))));
}

fn expect_remove_label(gh_client: &mut MockGHClient, ctx: &Ctx, number: i64, name: &'static str) {
    gh_client
        .expect_remove_label()
        .with(eq(ctx.clone()), eq(number), eq(name))
        .times(1)
        .returning(|_, _, _| Box::pin(future::ready(Ok(()))));
}

fn override_event(external_id: Option<&str>, pull_requests: &[(i64, i64)]) -> CheckRunEvent {
    CheckRunEvent {
        action: CheckRunEventAction::RequestedAction,
        check_run: CheckRunEventCheckRun {
            external_id: external_id.map(ToString::to_string),
            head_sha: "head_sha".to_string(),
            pull_requests: pull_requests
                .iter()
                .map(|(number, repo_id)| CheckRunEventPullRequest {
                    base: CheckRunEventPullRequestBase {
                        repo: CheckRunEventPullRequestRepository { id: *repo_id },
                    },
                    number: *number,
                })
                .collect(),
        },
        installation: Installation { id: 10 },
        repository: Repository {
            id: 20,
            name: "repo".to_string(),
            owner: RepositoryOwner {
                login: "owner".to_string(),
            },
        },
        requested_action: Some(RequestedAction {
            identifier: OVERRIDE_ACTION_IDENTIFIER.to_string(),
        }),
    }
}

fn pr_opened_event(labels: &[&str]) -> PullRequestEvent {
    PullRequestEvent {
        action: PullRequestEventAction::Opened,
        installation: Installation { id: 10 },
        number: 42,
        organization: None,
        pull_request: PullRequest {
            base: PullRequestBase {
                ref_: "base_ref".to_string(),
                sha: "base_sha".to_string(),
            },
            head: PullRequestHead {
                ref_: "head_ref".to_string(),
                sha: "head_sha".to_string(),
            },
            html_url: "url".to_string(),
            labels: labels
                .iter()
                .map(|name| PullRequestLabel {
                    name: name.to_string(),
                })
                .collect(),
        },
        repository: Repository {
            id: 20,
            name: "repo".to_string(),
            owner: RepositoryOwner {
                login: "owner".to_string(),
            },
        },
    }
}

fn pr_state(head_sha: &str, is_open: bool, labels: &[&str]) -> PullRequestState {
    PullRequestState {
        head_sha: head_sha.to_string(),
        is_open,
        labels: labels.iter().map(ToString::to_string).collect(),
    }
}

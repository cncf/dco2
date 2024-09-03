use crate::{
    dco::{
        event::{
            CHECK_NAME, OVERRIDE_ACTION_DESCRIPTION, OVERRIDE_ACTION_IDENTIFIER, OVERRIDE_ACTION_LABEL,
            OVERRIDE_ACTION_SUMMARY,
        },
        process_event,
    },
    github::{
        CheckRunAction, CheckRunConclusion, CheckRunEvent, CheckRunEventAction, CheckRunEventCheckRun,
        CheckRunStatus, Commit, Event, GitUser, Installation, MockGHClient, PullRequest, PullRequestBase,
        PullRequestEvent, PullRequestEventAction, PullRequestHead, Repository, RequestedAction,
    },
};
use anyhow::anyhow;
use indoc::indoc;
use mockall::predicate::eq;
use std::{future, sync::Arc};

#[tokio::test]
async fn check_run_event_other_action() {
    let event = CheckRunEvent {
        action: CheckRunEventAction::Other,
        check_run: CheckRunEventCheckRun {
            head_sha: "head_sha".to_string(),
        },
        installation: Installation { id: 1 },
        repository: Repository {
            full_name: "owner/repo".to_string(),
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
            head_sha: "head_sha".to_string(),
        },
        installation: Installation { id: 1 },
        repository: Repository {
            full_name: "owner/repo".to_string(),
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
            head_sha: "head_sha".to_string(),
        },
        installation: Installation { id: 1 },
        repository: Repository {
            full_name: "owner/repo".to_string(),
        },
        requested_action: Some(RequestedAction {
            identifier: OVERRIDE_ACTION_IDENTIFIER.to_string(),
        }),
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
                && check_run.summary() == OVERRIDE_ACTION_SUMMARY
        })
        .times(1)
        .returning(|_, _| Box::pin(future::ready(Err(anyhow!("test error")))));

    process_event(Arc::new(gh_client), &Event::CheckRun(event)).await.unwrap();
}

#[tokio::test]
async fn check_run_event_requested_action_override_success() {
    let event = CheckRunEvent {
        action: CheckRunEventAction::RequestedAction,
        check_run: CheckRunEventCheckRun {
            head_sha: "head_sha".to_string(),
        },
        installation: Installation { id: 1 },
        repository: Repository {
            full_name: "owner/repo".to_string(),
        },
        requested_action: Some(RequestedAction {
            identifier: OVERRIDE_ACTION_IDENTIFIER.to_string(),
        }),
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
                && check_run.summary() == OVERRIDE_ACTION_SUMMARY
        })
        .times(1)
        .returning(|_, _| Box::pin(future::ready(Ok(()))));

    process_event(Arc::new(gh_client), &Event::CheckRun(event)).await.unwrap();
}

#[tokio::test]
async fn pull_request_event_other_action() {
    let event = PullRequestEvent {
        action: PullRequestEventAction::Other,
        installation: Installation { id: 1 },
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
        },
        repository: Repository {
            full_name: "owner/repo".to_string(),
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
        },
        repository: Repository {
            full_name: "owner/repo".to_string(),
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
#[should_panic(expected = "error creating check run")]
async fn pull_request_event_opened_action_error_creating_check_run() {
    let event = PullRequestEvent {
        action: PullRequestEventAction::Opened,
        installation: Installation { id: 1 },
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
        },
        repository: Repository {
            full_name: "owner/repo".to_string(),
        },
    };

    let mut gh_client = MockGHClient::new();
    gh_client
        .expect_compare_commits()
        .with(eq(event.ctx()), eq("base_sha"), eq("head_sha"))
        .times(1)
        .returning(|_, _, _| {
            Box::pin(future::ready(Ok(vec![Commit {
                author: Some(GitUser {
                    name: "user1".to_string(),
                    email: "user1@email.test".to_string(),
                    ..Default::default()
                }),
                committer: Some(GitUser {
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
        },
        repository: Repository {
            full_name: "owner/repo".to_string(),
        },
    };

    let mut gh_client = MockGHClient::new();
    gh_client
        .expect_compare_commits()
        .with(eq(event.ctx()), eq("base_sha"), eq("head_sha"))
        .times(1)
        .returning(|_, _, _| {
            Box::pin(future::ready(Ok(vec![Commit {
                author: Some(GitUser {
                    name: "user1".to_string(),
                    email: "user1@email.test".to_string(),
                    ..Default::default()
                }),
                committer: Some(GitUser {
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
        },
        repository: Repository {
            full_name: "owner/repo".to_string(),
        },
    };

    let mut gh_client = MockGHClient::new();
    gh_client
        .expect_compare_commits()
        .with(eq(event.ctx()), eq("base_sha"), eq("head_sha"))
        .times(1)
        .returning(|_, _, _| {
            Box::pin(future::ready(Ok(vec![Commit {
                author: Some(GitUser {
                    name: "user1".to_string(),
                    email: "user1@email.test".to_string(),
                    ..Default::default()
                }),
                committer: Some(GitUser {
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
        })
        .times(1)
        .returning(|_, _| Box::pin(future::ready(Ok(()))));

    process_event(Arc::new(gh_client), &Event::PullRequest(event)).await.unwrap();
}

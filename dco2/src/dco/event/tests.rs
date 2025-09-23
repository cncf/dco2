use std::{future, sync::Arc};

use anyhow::{Ok, anyhow};
use indoc::indoc;
use mockall::predicate::eq;

use crate::{
    dco::{
        event::{
            CHECK_FAILED_TITLE, CHECK_NAME, CHECK_PASSED_TITLE, MERGE_GROUP_CHECKS_REQUESTED_SUMMARY,
            OVERRIDE_ACTION_DESCRIPTION, OVERRIDE_ACTION_IDENTIFIER, OVERRIDE_ACTION_LABEL,
            OVERRIDE_ACTION_SUMMARY,
        },
        process_event,
    },
    github::{
        CheckRunAction, CheckRunConclusion, CheckRunEvent, CheckRunEventAction, CheckRunEventCheckRun,
        CheckRunStatus, Commit, Config, ConfigRequire, Event, Installation, MergeGroupEvent,
        MergeGroupEventAction, MergeGroupEventMergeGroup, MergeGroupHeadCommit, MockGHClient, Organization,
        PullRequest, PullRequestBase, PullRequestEvent, PullRequestEventAction, PullRequestHead, Repository,
        RepositoryOwner, RequestedAction, User,
    },
};

#[tokio::test]
async fn check_run_event_other_action() {
    let event = CheckRunEvent {
        action: CheckRunEventAction::Other,
        check_run: CheckRunEventCheckRun {
            head_sha: "head_sha".to_string(),
        },
        installation: Installation { id: 1 },
        repository: Repository {
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
            head_sha: "head_sha".to_string(),
        },
        installation: Installation { id: 1 },
        repository: Repository {
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
            head_sha: "head_sha".to_string(),
        },
        installation: Installation { id: 1 },
        repository: Repository {
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
async fn check_run_event_requested_action_override_success() {
    let event = CheckRunEvent {
        action: CheckRunEventAction::RequestedAction,
        check_run: CheckRunEventCheckRun {
            head_sha: "head_sha".to_string(),
        },
        installation: Installation { id: 1 },
        repository: Repository {
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
        })
        .times(1)
        .returning(|_, _| Box::pin(future::ready(Ok(()))));

    process_event(Arc::new(gh_client), &Event::MergeGroup(event)).await.unwrap();
}

#[tokio::test]
async fn pull_request_event_other_action() {
    let event = PullRequestEvent {
        action: PullRequestEventAction::Other,
        installation: Installation { id: 1 },
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
        },
        repository: Repository {
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
        },
        repository: Repository {
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
        },
        repository: Repository {
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
        },
        repository: Repository {
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
        },
        repository: Repository {
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
        },
        repository: Repository {
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
        },
        repository: Repository {
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
async fn pull_request_event_opened_action_success_check_failed() {
    let event = PullRequestEvent {
        action: PullRequestEventAction::Opened,
        installation: Installation { id: 1 },
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
        },
        repository: Repository {
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

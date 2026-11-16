use std::sync::{Arc, Mutex};

use axum::response::{IntoResponse, Response};
use chrono::TimeZone;
use http::Method;
use indoc::indoc;
use pretty_assertions::assert_eq;
use serde_json::{Value, json};

use super::*;

/// Private key generated for these tests only. It does not belong to any
/// GitHub App and must never be used outside of them.
const TEST_APP_PRIVATE_KEY: &str = include_str!("../testdata/test-app-private-key.pem");

#[tokio::test]
async fn add_label_creates_missing_label() {
    // Setup mock GitHub server
    let server = MockGitHub::start(|req| match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/repos/owner/repo/labels/needs-dco") => not_found(),
        ("POST", "/repos/owner/repo/labels") => {
            (StatusCode::CREATED, axum::Json(label_json("needs-dco"))).into_response()
        }
        ("POST", "/repos/owner/repo/issues/1/labels") => {
            axum::Json(json!([label_json("needs-dco")])).into_response()
        }
        _ => unexpected(req),
    })
    .await;
    let gh = gh_client(Some(&server.url));

    // Add the label
    gh.add_label(&ctx("owner", "repo"), 1, "needs-dco").await.unwrap();

    // Check the label was created before being added to the pull request
    let requests = server.requests();
    assert_eq!(
        request_lines(&requests),
        vec![
            "GET /repos/owner/repo/labels/needs-dco",
            "POST /repos/owner/repo/labels",
            "POST /repos/owner/repo/issues/1/labels",
        ]
    );
    assert_eq!(
        requests[1].json_body(),
        json!({
            "color": "ededed",
            "description": "DCO check requires attention",
            "name": "needs-dco",
        })
    );
    assert_eq!(requests[2].json_body(), json!({"labels": ["needs-dco"]}));
}

#[tokio::test]
async fn add_label_ignores_label_created_concurrently() {
    // Setup mock GitHub server (label creation fails as it already exists)
    let server = MockGitHub::start(|req| match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/repos/owner/repo/labels/needs-dco") => not_found(),
        ("POST", "/repos/owner/repo/labels") => {
            let body = json!({"message": "Validation Failed"});
            (StatusCode::UNPROCESSABLE_ENTITY, axum::Json(body)).into_response()
        }
        ("POST", "/repos/owner/repo/issues/1/labels") => {
            axum::Json(json!([label_json("needs-dco")])).into_response()
        }
        _ => unexpected(req),
    })
    .await;
    let gh = gh_client(Some(&server.url));

    // Add the label
    gh.add_label(&ctx("owner", "repo"), 1, "needs-dco").await.unwrap();

    // Check the label was still added to the pull request
    assert_eq!(
        request_lines(&server.requests()),
        vec![
            "GET /repos/owner/repo/labels/needs-dco",
            "POST /repos/owner/repo/labels",
            "POST /repos/owner/repo/issues/1/labels",
        ]
    );
}

#[test]
fn app_config_validate_accepts_webhook_secret() {
    assert!(app_config(" secret ").validate().is_ok());
}

#[test]
fn app_config_validate_rejects_blank_webhook_secret() {
    for webhook_secret in ["", " \t\n"] {
        let err = app_config(webhook_secret).validate().unwrap_err();
        assert_eq!(
            err.to_string(),
            "github app webhook secret cannot be empty",
            "webhook secret: {webhook_secret:?}"
        );
    }
}

#[test]
fn app_config_validate_rejects_quoted_webhook_secret() {
    for webhook_secret in ["\"\"", "\"secret\""] {
        let err = app_config(webhook_secret).validate().unwrap_err();
        assert_eq!(
            err.to_string(),
            "github app webhook secret cannot be wrapped in quotes",
            "webhook secret: {webhook_secret:?}"
        );
    }
    for webhook_secret in ["\"", "\"secret", "se\"cr\"et", "secret\""] {
        assert!(
            app_config(webhook_secret).validate().is_ok(),
            "webhook secret: {webhook_secret:?}"
        );
    }
}

#[tokio::test]
async fn can_manage_labels_caches_permissions() {
    // Setup mock GitHub server
    let server = MockGitHub::start(|req| match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/app/installations/1") => {
            let body = json!({"account": {}, "id": 1, "permissions": {"issues": "write"}});
            axum::Json(body).into_response()
        }
        _ => unexpected(req),
    })
    .await;
    let gh = gh_client(Some(&server.url));
    let ctx = ctx("owner", "repo");

    // Check permissions twice
    let granted1 = gh.can_manage_labels(&ctx).await.unwrap();
    let granted2 = gh.can_manage_labels(&ctx).await.unwrap();

    // Check permissions were granted and the installation requested once
    assert!(granted1);
    assert!(granted2);
    assert_eq!(
        request_lines(&server.requests()),
        vec!["GET /app/installations/1"]
    );
}

#[test]
fn check_run_new_truncates_fields_at_char_boundary() {
    // Setup input whose limits fall in the middle of a two bytes character
    let action = CheckRunAction {
        label: format!("{}é", "l".repeat(19)),
        description: format!("{}é", "d".repeat(39)),
        identifier: format!("{}é", "i".repeat(19)),
    };
    let summary = format!("{}é", "s".repeat(65534));

    // Create the check run
    let check_run = CheckRun::new(new_check_run_input(vec![action], &summary));

    // Check the characters crossing the limits were dropped
    assert_eq!(
        check_run.actions(),
        &[CheckRunAction {
            label: "l".repeat(19),
            description: "d".repeat(39),
            identifier: "i".repeat(19),
        }]
    );
    assert_eq!(check_run.summary(), "s".repeat(65534));
}

#[test]
fn check_run_new_truncates_fields_to_max_length() {
    // Setup input exceeding the limits by one byte
    let action = CheckRunAction {
        label: "l".repeat(21),
        description: "d".repeat(41),
        identifier: "i".repeat(21),
    };
    let summary = "s".repeat(65536);

    // Create the check run
    let check_run = CheckRun::new(new_check_run_input(vec![action], &summary));

    // Check the fields were truncated to the limits
    assert_eq!(
        check_run.actions(),
        &[CheckRunAction {
            label: "l".repeat(20),
            description: "d".repeat(40),
            identifier: "i".repeat(20),
        }]
    );
    assert_eq!(check_run.summary(), "s".repeat(65535));
}

#[test]
fn commit_from_commit_data_maps_fields() {
    let commit_data = commit_data(json!({
        "author": {"login": "bot[bot]", "type": "Bot"},
        "commit": {
            "author": {"email": "bot@email.test", "name": "bot"},
            "committer": {"email": "user1@email.test", "name": "user1"},
            "message": "Test commit message",
            "tree": {},
            "url": "",
            "verification": {"verified": true},
        },
        "committer": {"login": "user1", "type": "User"},
        "html_url": "https://github.com/owner/repo/commit/sha",
        "parents": [{"sha": "parent1"}, {"sha": "parent2"}],
        "sha": "sha",
    }));

    assert_eq!(
        Commit::from(commit_data),
        Commit {
            author: Some(User {
                name: "bot".to_string(),
                email: "bot@email.test".to_string(),
                is_bot: true,
                login: Some("bot[bot]".to_string()),
            }),
            committer: Some(User {
                name: "user1".to_string(),
                email: "user1@email.test".to_string(),
                is_bot: false,
                login: Some("user1".to_string()),
            }),
            html_url: "https://github.com/owner/repo/commit/sha".to_string(),
            is_merge: true,
            message: "Test commit message".to_string(),
            sha: "sha".to_string(),
            verified: Some(true),
        }
    );
}

#[test]
fn commit_from_commit_data_without_github_accounts() {
    let commit_data = commit_data(json!({
        "commit": {
            "author": {"email": "user1@email.test", "name": "user1"},
            "committer": {"email": "user2@email.test", "name": "user2"},
            "message": "Test commit message",
            "tree": {},
            "url": "",
        },
        "html_url": "https://github.com/owner/repo/commit/sha",
        "parents": [{"sha": "parent1"}],
        "sha": "sha",
    }));

    assert_eq!(
        Commit::from(commit_data),
        Commit {
            author: Some(User {
                name: "user1".to_string(),
                email: "user1@email.test".to_string(),
                is_bot: false,
                login: None,
            }),
            committer: Some(User {
                name: "user2".to_string(),
                email: "user2@email.test".to_string(),
                is_bot: false,
                login: None,
            }),
            html_url: "https://github.com/owner/repo/commit/sha".to_string(),
            is_merge: false,
            message: "Test commit message".to_string(),
            sha: "sha".to_string(),
            verified: None,
        }
    );
}

#[tokio::test]
async fn compare_commits_accepts_max_commits() {
    // Setup mock GitHub server with a comparison of the maximum size supported
    let shas: Vec<String> = (1..=MAX_COMPARE_COMMITS).map(|i| format!("sha{i}")).collect();
    let server = MockGitHub::start({
        let shas = shas.clone();
        move |req| match (req.method.as_str(), req.path.as_str()) {
            ("GET", "/repos/owner/compare-max-commits/compare/base_sha...head_sha") => {
                compare_commits_response(&shas, shas.len(), req)
            }
            _ => unexpected(req),
        }
    })
    .await;
    let gh = gh_client(Some(&server.url));

    // Compare commits
    let commits = gh
        .compare_commits(&ctx("owner", "compare-max-commits"), "base_sha", "head_sha")
        .await
        .unwrap();

    // Check all commits were returned without requesting more pages than needed
    assert_eq!(commits.into_iter().map(|c| c.sha).collect::<Vec<_>>(), shas);
    assert_eq!(server.requests().len(), MAX_COMPARE_COMMITS / 100);
}

#[tokio::test]
async fn compare_commits_fetches_all_pages() {
    // Setup mock GitHub server with a comparison longer than the 250 commits
    // returned when no paging parameters are provided
    let shas: Vec<String> = (1..=300).map(|i| format!("sha{i}")).collect();
    let server = MockGitHub::start({
        let shas = shas.clone();
        move |req| match (req.method.as_str(), req.path.as_str()) {
            ("GET", "/repos/owner/compare-all-pages/compare/base_sha...head_sha") => {
                compare_commits_response(&shas, shas.len(), req)
            }
            _ => unexpected(req),
        }
    })
    .await;
    let gh = gh_client(Some(&server.url));

    // Compare commits
    let commits = gh
        .compare_commits(&ctx("owner", "compare-all-pages"), "base_sha", "head_sha")
        .await
        .unwrap();

    // Check all commits were returned in chronological order
    assert_eq!(commits.into_iter().map(|c| c.sha).collect::<Vec<_>>(), shas);

    // Check the pages requested
    assert_eq!(
        server.requests().iter().map(|req| req.query.as_str()).collect::<Vec<_>>(),
        vec![
            "page=1&per_page=100",
            "page=2&per_page=100",
            "page=3&per_page=100"
        ]
    );
}

#[tokio::test]
async fn compare_commits_fetches_all_pages_when_total_commits_not_reported() {
    // Setup mock GitHub server whose comparison does not report the total
    // number of commits
    let shas: Vec<String> = (1..=300).map(|i| format!("sha{i}")).collect();
    let server = MockGitHub::start({
        let shas = shas.clone();
        move |req| match (req.method.as_str(), req.path.as_str()) {
            ("GET", "/repos/owner/compare-no-total/compare/base_sha...head_sha") => {
                compare_commits_response(&shas, 0, req)
            }
            _ => unexpected(req),
        }
    })
    .await;
    let gh = gh_client(Some(&server.url));

    // Compare commits
    let commits = gh
        .compare_commits(&ctx("owner", "compare-no-total"), "base_sha", "head_sha")
        .await
        .unwrap();

    // Check all commits were returned, requesting pages until one is not full
    assert_eq!(commits.into_iter().map(|c| c.sha).collect::<Vec<_>>(), shas);
    assert_eq!(
        server.requests().iter().map(|req| req.query.as_str()).collect::<Vec<_>>(),
        vec![
            "page=1&per_page=100",
            "page=2&per_page=100",
            "page=3&per_page=100",
            "page=4&per_page=100"
        ]
    );
}

#[tokio::test]
async fn compare_commits_rejects_too_many_commits() {
    // Setup mock GitHub server with a comparison exceeding the maximum number
    // of commits supported
    let shas: Vec<String> = (1..=MAX_COMPARE_COMMITS + 1).map(|i| format!("sha{i}")).collect();
    let server = MockGitHub::start(move |req| match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/repos/owner/compare-too-many/compare/base_sha...head_sha") => {
            compare_commits_response(&shas, shas.len(), req)
        }
        _ => unexpected(req),
    })
    .await;
    let gh = gh_client(Some(&server.url));

    // Compare commits
    let err = gh
        .compare_commits(&ctx("owner", "compare-too-many"), "base_sha", "head_sha")
        .await
        .unwrap_err();

    // Check the comparison was rejected after requesting the first page only
    assert_eq!(
        err.downcast_ref::<TooManyCommitsError>(),
        Some(&TooManyCommitsError {
            max: MAX_COMPARE_COMMITS
        })
    );
    assert_eq!(server.requests().len(), 1);
}

#[tokio::test]
async fn compare_commits_rejects_too_many_commits_when_total_commits_not_reported() {
    // Setup mock GitHub server with a comparison exceeding the maximum number
    // of commits supported that does not report the total number of commits
    let shas: Vec<String> = (1..=MAX_COMPARE_COMMITS + 1).map(|i| format!("sha{i}")).collect();
    let server = MockGitHub::start(move |req| match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/repos/owner/compare-too-many-no-total/compare/base_sha...head_sha") => {
            compare_commits_response(&shas, 0, req)
        }
        _ => unexpected(req),
    })
    .await;
    let gh = gh_client(Some(&server.url));

    // Compare commits
    let err = gh
        .compare_commits(&ctx("owner", "compare-too-many-no-total"), "base_sha", "head_sha")
        .await
        .unwrap_err();

    // Check the comparison was rejected once the maximum was exceeded
    assert_eq!(
        err.downcast_ref::<TooManyCommitsError>(),
        Some(&TooManyCommitsError {
            max: MAX_COMPARE_COMMITS
        })
    );
    assert_eq!(server.requests().len(), MAX_COMPARE_COMMITS / 100 + 1);
}

#[tokio::test]
async fn compare_commits_stops_at_last_page() {
    // Setup mock GitHub server whose comparison reports more commits than the
    // ones it actually returns
    let shas: Vec<String> = (1..=150).map(|i| format!("sha{i}")).collect();
    let server = MockGitHub::start({
        let shas = shas.clone();
        move |req| match (req.method.as_str(), req.path.as_str()) {
            ("GET", "/repos/owner/compare-last-page/compare/base_sha...head_sha")
                if req.query_param("page").as_deref() != Some("3") =>
            {
                compare_commits_response(&shas, 300, req)
            }
            _ => unexpected(req),
        }
    })
    .await;
    let gh = gh_client(Some(&server.url));

    // Compare commits
    let commits = gh
        .compare_commits(&ctx("owner", "compare-last-page"), "base_sha", "head_sha")
        .await
        .unwrap();

    // Check the commits returned and that no pages were requested after the
    // first one with fewer commits than requested (the third page would fail)
    assert_eq!(commits.into_iter().map(|c| c.sha).collect::<Vec<_>>(), shas);
    assert_eq!(
        server.requests().iter().map(|req| req.query.as_str()).collect::<Vec<_>>(),
        vec!["page=1&per_page=100", "page=2&per_page=100"]
    );
}

#[test]
fn config_accessors_use_defaults_when_fields_missing() {
    let config: Config = serde_yaml::from_str(indoc! {"
        allowRemediationCommits:
          individual: true
        require: {}
    "})
    .unwrap();

    assert!(config.individual_remediation_commits_are_allowed());
    assert!(!config.third_party_remediation_commits_are_allowed());
    assert!(config.members_signoff_is_required());
}

#[test]
fn config_accessors_use_defaults_when_sections_missing() {
    for yaml in ["label: needs-dco", "{}", "allowRemediationCommits:\nrequire:\n"] {
        let config: Config = serde_yaml::from_str(yaml).unwrap();

        assert!(config.override_action_is_allowed(), "{yaml}");
        assert!(!config.individual_remediation_commits_are_allowed(), "{yaml}");
        assert!(!config.third_party_remediation_commits_are_allowed(), "{yaml}");
        assert!(config.members_signoff_is_required(), "{yaml}");
    }
}

#[test]
fn config_label_absent() {
    let config = Config::default();
    assert_eq!(config.label(), None);
}

#[test]
fn config_label_blank() {
    let config = config_with_label("  ");
    assert_eq!(config.label(), None);
}

#[test]
fn config_label_dot_segment() {
    assert_eq!(config_with_label(".").label(), None);
    assert_eq!(config_with_label("..").label(), None);
}

#[test]
fn config_label_max_length() {
    let label = "a".repeat(50);
    let config = config_with_label(&label);
    assert_eq!(config.label(), Some(label.as_str()));
}

#[test]
fn config_label_max_length_counts_characters() {
    let label = "é".repeat(50);
    assert_eq!(config_with_label(&label).label(), Some(label.as_str()));
    assert_eq!(config_with_label(&"é".repeat(51)).label(), None);
}

#[test]
fn config_label_too_long() {
    let config = config_with_label(&"a".repeat(51));
    assert_eq!(config.label(), None);
}

#[test]
fn config_label_trimmed() {
    let config = config_with_label(" needs-dco ");
    assert_eq!(config.label(), Some("needs-dco"));
}

#[test]
fn config_label_yaml() {
    let config: Config = serde_yaml::from_str("label: needs-dco").unwrap();
    assert_eq!(config.label(), Some("needs-dco"));
}

#[test]
fn config_yaml_reference_file_matches_defaults() {
    let config: Option<Config> =
        serde_yaml::from_str(include_str!("../../../../docs/config/dco.yml")).unwrap();
    assert_eq!(config, Some(Config::default()));
}

#[tokio::test]
async fn create_check_run_request_body_action_required() {
    // Setup mock GitHub server and check run
    let server = MockGitHub::start(|req| match (req.method.as_str(), req.path.as_str()) {
        ("POST", "/repos/owner/repo/check-runs") => check_run_created(),
        _ => unexpected(req),
    })
    .await;
    let gh = gh_client(Some(&server.url));
    let check_run = CheckRun::new(NewCheckRunInput {
        actions: vec![CheckRunAction {
            label: "Set DCO to pass".to_string(),
            description: "Manually set DCO check result to passed".to_string(),
            identifier: "override".to_string(),
        }],
        completed_at: timestamp(5),
        conclusion: CheckRunConclusion::ActionRequired,
        external_id: Some("42".to_string()),
        head_sha: "head_sha".to_string(),
        name: "DCO".to_string(),
        started_at: timestamp(0),
        status: CheckRunStatus::Completed,
        summary: "summary".to_string(),
        title: "Check failed".to_string(),
    });

    // Create the check run
    gh.create_check_run(&ctx("owner", "repo"), &check_run).await.unwrap();

    // Check the request body sent
    let requests = server.requests();
    assert_eq!(
        request_lines(&requests),
        vec!["POST /repos/owner/repo/check-runs"]
    );
    assert_eq!(
        requests[0].json_body(),
        json!({
            "actions": [{
                "description": "Manually set DCO check result to passed",
                "identifier": "override",
                "label": "Set DCO to pass",
            }],
            "completed_at": "1970-01-01T00:00:05Z",
            "conclusion": "action_required",
            "external_id": "42",
            "head_sha": "head_sha",
            "name": "DCO",
            "output": {"summary": "summary", "title": "Check failed"},
            "started_at": "1970-01-01T00:00:00Z",
            "status": "completed",
        })
    );
}

#[tokio::test]
async fn create_check_run_request_body_success() {
    // Setup mock GitHub server and check run
    let server = MockGitHub::start(|req| match (req.method.as_str(), req.path.as_str()) {
        ("POST", "/repos/owner/repo/check-runs") => check_run_created(),
        _ => unexpected(req),
    })
    .await;
    let gh = gh_client(Some(&server.url));
    let check_run = CheckRun::new(NewCheckRunInput {
        actions: vec![],
        completed_at: timestamp(5),
        conclusion: CheckRunConclusion::Success,
        external_id: None,
        head_sha: "head_sha".to_string(),
        name: "DCO".to_string(),
        started_at: timestamp(0),
        status: CheckRunStatus::Completed,
        summary: "summary".to_string(),
        title: "Check passed!".to_string(),
    });

    // Create the check run
    gh.create_check_run(&ctx("owner", "repo"), &check_run).await.unwrap();

    // Check the request body sent (empty fields are omitted)
    let requests = server.requests();
    assert_eq!(
        request_lines(&requests),
        vec!["POST /repos/owner/repo/check-runs"]
    );
    assert_eq!(
        requests[0].json_body(),
        json!({
            "completed_at": "1970-01-01T00:00:05Z",
            "conclusion": "success",
            "head_sha": "head_sha",
            "name": "DCO",
            "output": {"summary": "summary", "title": "Check passed!"},
            "started_at": "1970-01-01T00:00:00Z",
            "status": "completed",
        })
    );
}

#[test]
fn encode_label_name_encodes_path_characters() {
    assert_eq!(
        encode_label_name("do-not-merge/dco%\\"),
        "do-not-merge%2Fdco%25%5C"
    );
}

#[test]
fn encode_label_name_keeps_other_characters() {
    assert_eq!(encode_label_name("needs dco"), "needs dco");
}

#[tokio::test]
async fn get_config_cache_keys_do_not_collide_for_hyphenated_names() {
    // Setup mock GitHub server (the configuration is cached per process, so
    // these owners and repositories must not be used in other tests)
    let server = MockGitHub::start(|req| match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/repos/collide-web/app/contents/.github/dco.yml") => not_found(),
        ("GET", "/repos/collide/web-app/contents/.github/dco.yml") => {
            content_file(&b64.encode("label: needs-dco\n"))
        }
        _ => unexpected(req),
    })
    .await;
    let gh = gh_client(Some(&server.url));

    // Get the configuration of both repositories
    let config1 = gh.get_config(&ctx("collide-web", "app")).await.unwrap();
    let config2 = gh.get_config(&ctx("collide", "web-app")).await.unwrap();

    // Check each repository got its own configuration
    assert_eq!(config1, None);
    assert_eq!(config2.as_ref().and_then(Config::label), Some("needs-dco"));
    assert_eq!(
        request_lines(&server.requests()),
        vec![
            "GET /repos/collide-web/app/contents/.github/dco.yml",
            "GET /repos/collide/web-app/contents/.github/dco.yml",
        ]
    );
}

#[tokio::test]
async fn get_config_requested_from_default_branch() {
    // Setup mock GitHub server (the configuration is cached per process, so
    // this owner and repository must not be used in other tests)
    let server = MockGitHub::start(|req| match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/repos/config-default-branch/repo/contents/.github/dco.yml") => {
            content_file(&b64.encode("label: needs-dco\n"))
        }
        _ => unexpected(req),
    })
    .await;
    let gh = gh_client(Some(&server.url));

    // Get the configuration
    let config = gh.get_config(&ctx("config-default-branch", "repo")).await.unwrap();

    // Check the configuration was requested without a ref
    assert_eq!(config.as_ref().and_then(Config::label), Some("needs-dco"));
    let requests = server.requests();
    assert_eq!(
        request_lines(&requests),
        vec!["GET /repos/config-default-branch/repo/contents/.github/dco.yml"]
    );
    assert_eq!(requests[0].query, "");
}

#[tokio::test]
async fn is_organization_member_cache_keys_do_not_collide_for_hyphenated_names() {
    // Setup mock GitHub server (memberships are cached per process, so these
    // organizations and users must not be used in other tests)
    let server = MockGitHub::start(|req| match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/orgs/collide-sigs/members/alice") => StatusCode::NO_CONTENT.into_response(),
        ("GET", "/orgs/collide/members/sigs-alice") => not_found(),
        _ => unexpected(req),
    })
    .await;
    let gh = gh_client(Some(&server.url));
    let ctx = ctx("owner", "repo");

    // Check both memberships
    let is_member1 = gh.is_organization_member(&ctx, "collide-sigs", "alice").await.unwrap();
    let is_member2 = gh.is_organization_member(&ctx, "collide", "sigs-alice").await.unwrap();

    // Check each membership was resolved independently
    assert!(is_member1);
    assert!(!is_member2);
    assert_eq!(
        request_lines(&server.requests()),
        vec![
            "GET /orgs/collide-sigs/members/alice",
            "GET /orgs/collide/members/sigs-alice",
        ]
    );
}

#[test]
fn is_permissions_error_not_found() {
    let err = http_error(StatusCode::NOT_FOUND, "Not Found");
    assert!(!is_permissions_error(&err));
}

#[test]
fn is_permissions_error_permissions_not_granted() {
    let err = http_error(
        StatusCode::FORBIDDEN,
        r#"{"message":"Resource not accessible by integration","status":"403"}"#,
    );
    assert!(is_permissions_error(&err));
}

#[test]
fn is_permissions_error_secondary_rate_limit() {
    let err = http_error(
        StatusCode::FORBIDDEN,
        r#"{"message":"You have exceeded a secondary rate limit","status":"403"}"#,
    );
    assert!(!is_permissions_error(&err));
}

#[test]
fn is_rate_limit_error_forbidden_with_rate_limit_message() {
    let err = http_error(StatusCode::FORBIDDEN, "You have exceeded a secondary rate limit.");
    assert!(is_rate_limit_error(
        &anyhow::Error::new(err).context("error adding label")
    ));
}

#[test]
fn is_rate_limit_error_forbidden_with_rate_limit_remaining_zero() {
    let err = http_error_with_header(StatusCode::FORBIDDEN, RATE_LIMIT_REMAINING_HEADER, "0");
    assert!(is_rate_limit_error(&anyhow::Error::new(err)));
}

#[test]
fn is_rate_limit_error_forbidden_with_remaining_requests() {
    let err = http_error_with_header(StatusCode::FORBIDDEN, RATE_LIMIT_REMAINING_HEADER, "4999");
    assert!(!is_rate_limit_error(&anyhow::Error::new(err)));
}

#[test]
fn is_rate_limit_error_forbidden_with_retry_after_header() {
    let err = http_error_with_header(StatusCode::FORBIDDEN, RETRY_AFTER.as_str(), "60");
    assert!(is_rate_limit_error(&anyhow::Error::new(err)));
}

#[test]
fn is_rate_limit_error_forbidden_without_rate_limit_details() {
    let err = http_error(StatusCode::FORBIDDEN, PERMISSIONS_NOT_GRANTED_MESSAGE);
    assert!(!is_rate_limit_error(&anyhow::Error::new(err)));
}

#[test]
fn is_rate_limit_error_other_error() {
    assert!(!is_rate_limit_error(&anyhow::anyhow!("test error")));
}

#[test]
fn is_rate_limit_error_primary_rate_limit() {
    let err = octorust::ClientError::RateLimited { duration: 60 };
    assert!(is_rate_limit_error(&anyhow::Error::new(err)));
}

#[test]
fn is_rate_limit_error_too_many_requests() {
    let err = http_error(StatusCode::TOO_MANY_REQUESTS, "");
    assert!(is_rate_limit_error(&anyhow::Error::new(err)));
}

#[test]
fn labels_error_permissions_error_evicts_installation() {
    // Setup cached clients and permissions for two installations
    let gh = gh_client(None);
    for inst_id in [1, 2] {
        gh.setup_client(inst_id).unwrap();
        gh.labels_permissions.lock().unwrap().cache_set(inst_id, true);
    }

    // Convert a permissions error returned for the first installation
    let err = gh.labels_error(
        1,
        http_error(StatusCode::FORBIDDEN, PERMISSIONS_NOT_GRANTED_MESSAGE),
    );

    // Check only the first installation was evicted from both caches
    assert!(err.is::<LabelsPermissionError>());
    let mut clients = gh.clients.lock().unwrap();
    assert!(clients.cache_get(&1).is_none());
    assert!(clients.cache_get(&2).is_some());
    let mut labels_permissions = gh.labels_permissions.lock().unwrap();
    assert_eq!(labels_permissions.cache_get(&1), None);
    assert_eq!(labels_permissions.cache_get(&2), Some(&true));
}

#[test]
fn labels_permissions_granted_issues_write() {
    let permissions = app_permissions(r#"{"issues": "write", "metadata": "read"}"#);
    assert!(labels_permissions_granted(&permissions));
}

#[test]
fn labels_permissions_granted_not_granted() {
    let permissions = app_permissions(r#"{"checks": "write", "metadata": "read"}"#);
    assert!(!labels_permissions_granted(&permissions));
}

#[test]
fn labels_permissions_granted_pull_requests_write() {
    let permissions = app_permissions(r#"{"metadata": "read", "pull_requests": "write"}"#);
    assert!(labels_permissions_granted(&permissions));
}

#[test]
fn labels_permissions_granted_read_only() {
    let permissions = app_permissions(r#"{"issues": "read", "pull_requests": "read"}"#);
    assert!(!labels_permissions_granted(&permissions));
}

#[tokio::test]
async fn new_octorust_client_does_not_retry_too_many_requests() {
    // Setup mock GitHub server
    let server = MockGitHub::start(|req| match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/repos/owner/repo/pulls/1") => {
            let headers = [
                (RATE_LIMIT_REMAINING_HEADER, "0"),
                ("x-ratelimit-reset", "4102444800"),
            ];
            (StatusCode::TOO_MANY_REQUESTS, headers).into_response()
        }
        _ => unexpected(req),
    })
    .await;
    let gh = gh_client(Some(&server.url));

    // Get the pull request
    let err = gh.get_pull_request(&ctx("owner", "repo"), 1).await.unwrap_err();

    // Check the rate limited request was sent only once
    assert!(is_rate_limit_error(&err));
    assert_eq!(
        request_lines(&server.requests()),
        vec!["GET /repos/owner/repo/pulls/1"]
    );
}

#[tokio::test]
async fn new_octorust_client_retries_server_errors() {
    // Setup mock GitHub server
    let server = MockGitHub::start(|req| match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/repos/owner/repo/pulls/1") => StatusCode::BAD_GATEWAY.into_response(),
        _ => unexpected(req),
    })
    .await;
    let gh = gh_client(Some(&server.url));

    // Get the pull request
    let result = gh.get_pull_request(&ctx("owner", "repo"), 1).await;

    // Check the request failed after being retried the maximum number of times
    assert!(matches!(
        result.unwrap_err().downcast_ref::<octorust::ClientError>(),
        Some(octorust::ClientError::HttpError { status, .. }) if *status == StatusCode::BAD_GATEWAY
    ));
    assert_eq!(
        request_lines(&server.requests()),
        vec!["GET /repos/owner/repo/pulls/1"; MAX_RETRIES as usize + 1]
    );
}

#[tokio::test]
async fn remove_label_encodes_name_in_path() {
    // Setup mock GitHub server
    let server = MockGitHub::start(|req| match (req.method.as_str(), req.path.as_str()) {
        ("DELETE", "/repos/owner/repo/issues/1/labels/area%2Fdco%20100%25") => {
            axum::Json(json!([])).into_response()
        }
        _ => unexpected(req),
    })
    .await;
    let gh = gh_client(Some(&server.url));

    // Remove the label
    gh.remove_label(&ctx("owner", "repo"), 1, "area/dco 100%").await.unwrap();

    // Check the label name was encoded as a single path segment
    assert_eq!(
        request_lines(&server.requests()),
        vec!["DELETE /repos/owner/repo/issues/1/labels/area%2Fdco%20100%25"]
    );
}

#[test]
fn retry_strategy_forbidden_not_retried() {
    let result = Ok(response(StatusCode::FORBIDDEN));
    assert!(RetryStrategy.handle(&result) == Some(Retryable::Fatal));
}

#[test]
fn retry_strategy_server_error_retried() {
    let result = Ok(response(StatusCode::BAD_GATEWAY));
    assert!(RetryStrategy.handle(&result) == Some(Retryable::Transient));
}

#[test]
fn retry_strategy_success_not_retried() {
    let result = Ok(response(StatusCode::OK));
    assert!(RetryStrategy.handle(&result).is_none());
}

#[test]
fn retry_strategy_too_many_requests_not_retried() {
    let result = Ok(response(StatusCode::TOO_MANY_REQUESTS));
    assert!(RetryStrategy.handle(&result) == Some(Retryable::Fatal));
}

// Helpers.

/// Mock GitHub API server. Installation access token requests are answered
/// automatically and not recorded; the responder answers the others.
struct MockGitHub {
    /// Requests received (installation access token requests excluded).
    requests: Arc<Mutex<Vec<MockRequest>>>,
    /// Base URL of the server, used as the GitHub API host override.
    url: String,
}

impl MockGitHub {
    /// Start a new mock server listening on a random local port.
    async fn start<F>(responder: F) -> Self
    where
        F: Fn(&MockRequest) -> Response + Send + Sync + 'static,
    {
        // Setup router recording requests and answering them
        let requests = Arc::new(Mutex::new(vec![]));
        let responder = Arc::new(responder);
        let router = axum::Router::new().fallback({
            let requests = requests.clone();
            move |method: Method, uri: http::Uri, body: String| async move {
                let req = MockRequest {
                    body,
                    method,
                    path: uri.path().to_string(),
                    query: uri.query().unwrap_or_default().to_string(),
                };

                // Issue installation access tokens
                if req.method == Method::POST && req.path.ends_with("/access_tokens") {
                    let token = json!({"expires_at": "2099-01-01T00:00:00Z", "token": "test-token"});
                    return (StatusCode::CREATED, axum::Json(token)).into_response();
                }

                // Record and answer any other request
                requests.lock().unwrap().push(req.clone());
                responder(&req)
            }
        });

        // Start server
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });

        Self { requests, url }
    }

    /// Get the requests received so far.
    fn requests(&self) -> Vec<MockRequest> {
        self.requests.lock().unwrap().clone()
    }
}

/// Request received by the mock GitHub API server.
#[derive(Debug, Clone)]
struct MockRequest {
    /// Request body.
    body: String,
    /// Request method.
    method: Method,
    /// Request path (percent-encoded, as received).
    path: String,
    /// Request query string (empty when not provided).
    query: String,
}

impl MockRequest {
    /// Parse the request body as JSON.
    fn json_body(&self) -> Value {
        serde_json::from_str(&self.body).unwrap()
    }

    /// Get the value of the query parameter provided, if any.
    fn query_param(&self, name: &str) -> Option<String> {
        self.query
            .split('&')
            .filter_map(|pair| pair.split_once('='))
            .find(|(key, _)| *key == name)
            .map(|(_, value)| value.to_string())
    }
}

/// Build a GitHub application configuration with the webhook secret
/// provided.
fn app_config(webhook_secret: &str) -> AppConfig {
    AppConfig {
        api_host: None,
        app_id: 1,
        private_key: TEST_APP_PRIVATE_KEY.to_string(),
        webhook_secret: webhook_secret.to_string(),
    }
}

/// Parse the installation permissions provided.
fn app_permissions(json: &str) -> octorust::types::AppPermissions {
    serde_json::from_str(json).unwrap()
}

/// Response returned by GitHub when a check run is created.
fn check_run_created() -> Response {
    let check_run = json!({"id": 1, "name": "DCO", "output": {}, "status": "completed"});
    (StatusCode::CREATED, axum::Json(check_run)).into_response()
}

/// Parse the commit data provided.
fn commit_data(json: Value) -> octorust::types::CommitDataType {
    serde_json::from_value(json).unwrap()
}

/// Emulate GitHub's compare commits endpoint for a comparison including the
/// commits provided (in chronological order), reporting the total number of
/// commits given. Without paging parameters only the 250 most recent commits
/// are returned.
fn compare_commits_response(shas: &[String], total_commits: usize, req: &MockRequest) -> Response {
    const MAX_COMMITS_WITHOUT_PAGINATION: usize = 250;

    // Select the commits of the page requested
    let page = req.query_param("page").map(|p| p.parse::<usize>().unwrap());
    let per_page = req.query_param("per_page").map(|p| p.parse::<usize>().unwrap());
    let selected = if page.is_none() && per_page.is_none() {
        &shas[shas.len().saturating_sub(MAX_COMMITS_WITHOUT_PAGINATION)..]
    } else {
        let page = page.unwrap_or(1).max(1);
        let per_page = per_page.unwrap_or(30).min(100);
        let start = ((page - 1) * per_page).min(shas.len());
        let end = (start + per_page).min(shas.len());
        &shas[start..end]
    };

    // Prepare comparison
    let commit = |sha: &str| json!({"commit": {"message": "", "tree": {}, "url": ""}, "sha": sha});
    let comparison = json!({
        "base_commit": commit("base_sha"),
        "commits": selected.iter().map(|sha| commit(sha)).collect::<Vec<_>>(),
        "merge_base_commit": commit("base_sha"),
        "status": "ahead",
        "total_commits": total_commits,
    });
    axum::Json(comparison).into_response()
}

/// Build a configuration with the label provided.
fn config_with_label(label: &str) -> Config {
    Config {
        label: Some(label.to_string()),
        ..Default::default()
    }
}

/// Configuration file content response with the base64 content provided.
fn content_file(content: &str) -> Response {
    let body = json!({"_links": {}, "content": content, "encoding": "base64"});
    axum::Json(body).into_response()
}

/// Build a context for the repository provided (installation 1).
fn ctx(owner: &str, repo: &str) -> Ctx {
    Ctx {
        inst_id: 1,
        owner: owner.to_string(),
        repo: repo.to_string(),
    }
}

/// Build a GitHub client using the test private key and the API host
/// provided.
fn gh_client(api_host: Option<&str>) -> GHClientOctorust {
    GHClientOctorust::new(&AppConfig {
        api_host: api_host.map(ToString::to_string),
        app_id: 1,
        private_key: TEST_APP_PRIVATE_KEY.to_string(),
        webhook_secret: "secret".to_string(),
    })
    .unwrap()
}

/// Build an HTTP error with the status and error message provided.
fn http_error(status: StatusCode, error: &str) -> octorust::ClientError {
    octorust::ClientError::HttpError {
        status,
        headers: http::HeaderMap::new(),
        error: error.to_string(),
    }
}

/// Build an HTTP error with the status and header provided.
fn http_error_with_header(
    status: StatusCode,
    name: &'static str,
    value: &'static str,
) -> octorust::ClientError {
    let mut headers = http::HeaderMap::new();
    headers.insert(name, http::HeaderValue::from_static(value));
    octorust::ClientError::HttpError {
        status,
        headers,
        error: String::new(),
    }
}

/// Label JSON representation returned by GitHub.
fn label_json(name: &str) -> Value {
    json!({"name": name})
}

/// Build the input of a check run with the actions and summary provided.
fn new_check_run_input(actions: Vec<CheckRunAction>, summary: &str) -> NewCheckRunInput {
    NewCheckRunInput {
        actions,
        completed_at: timestamp(0),
        conclusion: CheckRunConclusion::Success,
        external_id: None,
        head_sha: "head_sha".to_string(),
        name: "DCO".to_string(),
        started_at: timestamp(0),
        status: CheckRunStatus::Completed,
        summary: summary.to_string(),
        title: "title".to_string(),
    }
}

/// Not found response returned by GitHub.
fn not_found() -> Response {
    let body = json!({"message": "Not Found"});
    (StatusCode::NOT_FOUND, axum::Json(body)).into_response()
}

/// Summarize the requests provided as `METHOD path` lines.
fn request_lines(requests: &[MockRequest]) -> Vec<String> {
    requests.iter().map(|req| format!("{} {}", req.method, req.path)).collect()
}

/// Build an HTTP response with the status provided.
fn response(status: StatusCode) -> reqwest::Response {
    http::Response::builder().status(status).body("").unwrap().into()
}

/// Build a timestamp the seconds provided after the Unix epoch.
fn timestamp(secs: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(secs, 0).unwrap()
}

/// Response returned by the mock server for requests not expected by a test.
fn unexpected(req: &MockRequest) -> Response {
    let body = format!("unexpected request: {} {}", req.method, req.path);
    (StatusCode::IM_A_TEAPOT, body).into_response()
}

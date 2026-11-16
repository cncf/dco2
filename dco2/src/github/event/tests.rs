use http::HeaderValue;

use super::*;

#[test]
fn check_run_event_actions_map_known_and_unknown_values() {
    let cases = [
        ("completed", CheckRunEventAction::Other),
        ("created", CheckRunEventAction::Other),
        ("requested_action", CheckRunEventAction::RequestedAction),
        ("rerequested", CheckRunEventAction::Rerequested),
        ("unknown", CheckRunEventAction::Other),
    ];

    for (value, expected_action) in cases {
        assert_eq!(
            parse_action::<CheckRunEventAction>(value),
            expected_action,
            "{value}"
        );
    }
}

#[test]
fn check_run_event_with_empty_external_id_and_without_pull_requests() {
    let body = r#"{
        "action": "requested_action",
        "check_run": {"external_id": "", "head_sha": "head_sha"},
        "installation": {"id": 1},
        "repository": {"id": 1, "name": "repo", "owner": {"login": "owner"}},
        "requested_action": {"identifier": "override"}
    }"#;

    let Event::CheckRun(event) = parse_event("check_run", body) else {
        panic!("unexpected event type");
    };

    assert_eq!(event.check_run.external_id.as_deref(), Some(""));
    assert!(event.check_run.pull_requests.is_empty());
}

#[test]
fn check_run_event_with_external_id_and_pull_requests() {
    let body = r#"{
        "action": "requested_action",
        "check_run": {
            "external_id": "123",
            "head_sha": "head_sha",
            "pull_requests": [{"base": {"repo": {"id": 1}}, "id": 1, "number": 123}]
        },
        "installation": {"id": 1},
        "repository": {"id": 1, "name": "repo", "owner": {"login": "owner"}},
        "requested_action": {"identifier": "override"}
    }"#;

    let Event::CheckRun(event) = parse_event("check_run", body) else {
        panic!("unexpected event type");
    };

    assert_eq!(event.action, CheckRunEventAction::RequestedAction);
    assert_eq!(event.check_run.external_id.as_deref(), Some("123"));
    assert_eq!(
        event.check_run.pull_requests,
        vec![CheckRunEventPullRequest {
            base: CheckRunEventPullRequestBase {
                repo: CheckRunEventPullRequestRepository { id: 1 },
            },
            number: 123,
        }]
    );
}

#[test]
fn check_run_event_without_external_id() {
    let body = r#"{
        "action": "requested_action",
        "check_run": {"head_sha": "head_sha"},
        "installation": {"id": 1},
        "repository": {"id": 1, "name": "repo", "owner": {"login": "owner"}},
        "requested_action": {"identifier": "override"}
    }"#;

    let Event::CheckRun(event) = parse_event("check_run", body) else {
        panic!("unexpected event type");
    };

    assert_eq!(event.check_run.external_id, None);
}

#[test]
fn merge_group_event_actions_map_known_and_unknown_values() {
    let cases = [
        ("checks_requested", MergeGroupEventAction::ChecksRequested),
        ("destroyed", MergeGroupEventAction::Other),
        ("unknown", MergeGroupEventAction::Other),
    ];

    for (value, expected_action) in cases {
        assert_eq!(
            parse_action::<MergeGroupEventAction>(value),
            expected_action,
            "{value}"
        );
    }
}

#[test]
fn merge_group_event_checks_requested() {
    let body = r#"{
        "action": "checks_requested",
        "installation": {"id": 3, "node_id": "I_3"},
        "merge_group": {
            "base_ref": "refs/heads/main",
            "base_sha": "base_sha",
            "head_commit": {
                "author": {"email": "octocat@github.com", "name": "Octocat"},
                "committer": {"email": "noreply@github.com", "name": "GitHub"},
                "id": "head_sha",
                "message": "Merge pull request #123 from owner/feature",
                "timestamp": "2024-01-01T00:00:00Z",
                "tree_id": "tree_sha"
            },
            "head_ref": "refs/heads/gh-readonly-queue/main/pr-123-base_sha",
            "head_sha": "head_sha"
        },
        "organization": {"id": 2, "login": "owner"},
        "repository": {
            "full_name": "owner/repo",
            "id": 1,
            "name": "repo",
            "node_id": "R_1",
            "owner": {"id": 2, "login": "owner", "type": "Organization"},
            "private": false
        },
        "sender": {"id": 4, "login": "octocat", "type": "User"}
    }"#;

    let Event::MergeGroup(event) = parse_event("merge_group", body) else {
        panic!("unexpected event type");
    };

    assert_eq!(event.action, MergeGroupEventAction::ChecksRequested);
    assert_eq!(event.installation.id, 3);
    assert_eq!(event.merge_group.head_commit.id, "head_sha");
    assert_eq!(event.repository.name, "repo");
    assert_eq!(event.repository.owner.login, "owner");
}

#[test]
fn missing_event_header_is_rejected() {
    let body = r#"{
        "action": "requested_action",
        "check_run": {"external_id": "123", "head_sha": "head_sha"},
        "installation": {"id": 1},
        "repository": {"id": 1, "name": "repo", "owner": {"login": "owner"}},
        "requested_action": {"identifier": "override"}
    }"#;

    assert_eq!(try_parse_event(None, body), Err(EventError::MissingHeader));
}

#[test]
fn pull_request_event_actions_map_known_and_unknown_values() {
    let cases = [
        ("closed", PullRequestEventAction::Other),
        ("edited", PullRequestEventAction::Other),
        ("opened", PullRequestEventAction::Opened),
        ("reopened", PullRequestEventAction::Other),
        ("synchronize", PullRequestEventAction::Synchronize),
        ("unknown", PullRequestEventAction::Other),
    ];

    for (value, expected_action) in cases {
        assert_eq!(
            parse_action::<PullRequestEventAction>(value),
            expected_action,
            "{value}"
        );
    }
}

#[test]
fn pull_request_event_with_labels() {
    let body = r#"{
        "action": "opened",
        "installation": {"id": 1},
        "number": 123,
        "pull_request": {
            "base": {"ref": "main", "sha": "base_sha"},
            "head": {"ref": "feature", "sha": "head_sha"},
            "html_url": "url",
            "labels": [{"id": 1, "name": "needs-dco", "color": "d73a4a"}]
        },
        "repository": {"id": 1, "name": "repo", "owner": {"login": "owner"}}
    }"#;

    let Event::PullRequest(event) = parse_event("pull_request", body) else {
        panic!("unexpected event type");
    };

    assert_eq!(event.action, PullRequestEventAction::Opened);
    assert_eq!(
        event.pull_request.labels,
        vec![PullRequestLabel {
            name: "needs-dco".to_string()
        }]
    );
}

#[test]
fn pull_request_event_with_organization() {
    let body = r#"{
        "action": "opened",
        "installation": {"id": 1},
        "number": 123,
        "organization": {"id": 1, "login": "org", "node_id": "O_1"},
        "pull_request": {
            "base": {"ref": "main", "sha": "base_sha"},
            "head": {"ref": "feature", "sha": "head_sha"},
            "html_url": "url"
        },
        "repository": {"id": 1, "name": "repo", "owner": {"login": "org"}}
    }"#;

    let Event::PullRequest(event) = parse_event("pull_request", body) else {
        panic!("unexpected event type");
    };

    assert_eq!(
        event.organization,
        Some(Organization {
            login: "org".to_string()
        })
    );
}

#[test]
fn pull_request_event_without_labels() {
    let body = r#"{
        "action": "opened",
        "installation": {"id": 1},
        "number": 123,
        "pull_request": {
            "base": {"ref": "main", "sha": "base_sha"},
            "head": {"ref": "feature", "sha": "head_sha"},
            "html_url": "url"
        },
        "repository": {"id": 1, "name": "repo", "owner": {"login": "owner"}}
    }"#;

    let Event::PullRequest(event) = parse_event("pull_request", body) else {
        panic!("unexpected event type");
    };

    assert_eq!(event.number, 123);
    assert!(event.pull_request.labels.is_empty());
}

#[test]
fn pull_request_event_without_organization() {
    let cases = [
        r#"{
            "action": "opened",
            "installation": {"id": 1},
            "number": 123,
            "pull_request": {
                "base": {"ref": "main", "sha": "base_sha"},
                "head": {"ref": "feature", "sha": "head_sha"},
                "html_url": "url"
            },
            "repository": {"id": 1, "name": "repo", "owner": {"login": "user"}}
        }"#,
        r#"{
            "action": "opened",
            "installation": {"id": 1},
            "number": 123,
            "organization": null,
            "pull_request": {
                "base": {"ref": "main", "sha": "base_sha"},
                "head": {"ref": "feature", "sha": "head_sha"},
                "html_url": "url"
            },
            "repository": {"id": 1, "name": "repo", "owner": {"login": "user"}}
        }"#,
    ];

    for body in cases {
        let Event::PullRequest(event) = parse_event("pull_request", body) else {
            panic!("unexpected event type");
        };

        assert_eq!(event.organization, None);
    }
}

#[test]
fn unsupported_event_is_rejected_without_parsing_payload() {
    for name in ["ping", "push"] {
        assert_eq!(
            try_parse_event(Some(name), "not json"),
            Err(EventError::UnsupportedEvent),
            "{name}"
        );
    }
}

// Helpers.

/// Deserialize an event action enum from its webhook string value.
fn parse_action<T: for<'de> Deserialize<'de>>(value: &str) -> T {
    serde_json::from_value(serde_json::Value::String(value.to_string())).unwrap()
}

/// Parse a webhook event that is expected to be valid.
fn parse_event(name: &str, body: &str) -> Event {
    try_parse_event(Some(name), body).unwrap()
}

/// Parse a webhook event, setting the event name header only when provided.
fn try_parse_event(name: Option<&str>, body: &str) -> Result<Event, EventError> {
    let mut headers = HeaderMap::new();
    if let Some(name) = name {
        headers.insert(EVENT_NAME_HEADER, HeaderValue::from_str(name).unwrap());
    }
    Event::try_from((&headers, &Bytes::from(body.to_string())))
}

use std::sync::{Arc, Mutex};

use anyhow::{Result, format_err};
use async_trait::async_trait;
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{HeaderMap, HeaderValue, Method, Request, StatusCode},
};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use tower::ServiceExt;

use dco2::github::{
    CheckRun, Commit, Config, Ctx, DynGHClient, EVENT_SIGNATURE_HEADER, GHClient, PullRequestState,
};

use super::{setup_router, verify_signature};

/// Body used in the signature verification tests.
const BODY: &[u8] = br#"{"action":"checks_requested"}"#;

/// Header containing the name of the webhook event.
const EVENT_NAME_HEADER: &str = "X-GitHub-Event";

/// Merge group checks requested event payload.
const MERGE_GROUP_PAYLOAD: &str = r#"{
    "action": "checks_requested",
    "installation": { "id": 1 },
    "merge_group": { "head_commit": { "id": "abc" } },
    "repository": { "id": 1, "name": "repo", "owner": { "login": "owner" } }
}"#;

/// Webhook secret configured in the router.
const SECRET: &str = "secret";

#[tokio::test]
async fn health_check_returns_ok() {
    // Setup router and request
    let router = setup_router(FakeGHClient::new().into_dyn(), SECRET);
    let request = Request::builder().uri("/health-check").body(Body::empty()).unwrap();

    // Send request and check the response
    assert_eq!(send(router, request).await, (StatusCode::OK, String::new()));
}

#[tokio::test]
async fn processed_event_returns_ok() {
    // Setup router and signed merge group event request
    let gh_client = FakeGHClient::new();
    let router = setup_router(gh_client.clone().into_dyn(), SECRET);
    let body = MERGE_GROUP_PAYLOAD.as_bytes();
    let request = webhook_request(Some("merge_group"), Some(&sign(SECRET, body)), body);

    // Send request and check the response
    assert_eq!(send(router, request).await, (StatusCode::OK, String::new()));

    // Check the event was processed creating a check run
    let ctx = Ctx {
        inst_id: 1,
        owner: "owner".to_string(),
        repo: "repo".to_string(),
    };
    assert_eq!(gh_client.check_runs(), vec![(ctx, "abc".to_string())]);
}

#[tokio::test]
async fn processing_error_returns_internal_server_error() {
    // Setup router (failing to create check runs) and signed event request
    let gh_client = FakeGHClient::failing();
    let router = setup_router(gh_client.clone().into_dyn(), SECRET);
    let body = MERGE_GROUP_PAYLOAD.as_bytes();
    let request = webhook_request(Some("merge_group"), Some(&sign(SECRET, body)), body);

    // Send request and check the response
    assert_eq!(
        send(router, request).await,
        (StatusCode::INTERNAL_SERVER_ERROR, String::new())
    );

    // Check the error came from processing the event
    assert_eq!(gh_client.check_runs().len(), 1);
}

#[tokio::test]
async fn signature_is_verified_before_parsing_event() {
    // Setup router
    let gh_client = FakeGHClient::new();
    let router = setup_router(gh_client.clone().into_dyn(), SECRET);

    // Send unsigned requests (unsupported event and no event header)
    for event in [Some("push"), None] {
        let request = webhook_request(event, None, b"{}");
        assert_eq!(
            send(router.clone(), request).await,
            (StatusCode::BAD_REQUEST, "no valid signature found".to_string()),
            "event: {event:?}"
        );
    }

    // Check no events were processed
    assert!(gh_client.check_runs().is_empty());
}

#[test]
fn signature_with_wrong_secret_or_tampered_body_fails() {
    // Check a signature generated with another secret fails
    let headers = signature_headers(&sign("other-secret", BODY));
    assert!(verify_signature(SECRET.as_bytes(), &headers, BODY).is_err());

    // Check a signature generated for another body fails
    let headers = signature_headers(&sign(SECRET, BODY));
    assert!(verify_signature(SECRET.as_bytes(), &headers, br#"{"action":"other"}"#).is_err());
}

#[test]
fn truncated_signature_fails() {
    // Check a signature missing its last byte fails
    let signature = sign(SECRET, BODY);
    let headers = signature_headers(&signature[..signature.len() - 2]);
    assert!(verify_signature(SECRET.as_bytes(), &headers, BODY).is_err());

    // Check an empty signature fails
    let headers = signature_headers("sha256=");
    assert!(verify_signature(SECRET.as_bytes(), &headers, BODY).is_err());
}

#[tokio::test]
async fn unsupported_event_returns_ok_without_processing() {
    // Setup router and signed unsupported event request
    let gh_client = FakeGHClient::new();
    let router = setup_router(gh_client.clone().into_dyn(), SECRET);
    let request = webhook_request(Some("push"), Some(&sign(SECRET, b"{}")), b"{}");

    // Send request and check the response
    assert_eq!(send(router, request).await, (StatusCode::OK, String::new()));

    // Check the event was not processed
    assert!(gh_client.check_runs().is_empty());
}

#[test]
fn valid_signature_passes() {
    let headers = signature_headers(&sign(SECRET, BODY));
    assert!(verify_signature(SECRET.as_bytes(), &headers, BODY).is_ok());
}

#[tokio::test]
async fn webhook_with_invalid_payload_returns_bad_request() {
    // Setup router and signed request with an invalid merge group payload
    let gh_client = FakeGHClient::new();
    let router = setup_router(gh_client.clone().into_dyn(), SECRET);
    let request = webhook_request(Some("merge_group"), Some(&sign(SECRET, b"{}")), b"{}");

    // Send request and check the response
    assert_eq!(
        send(router, request).await,
        (StatusCode::BAD_REQUEST, "invalid payload".to_string())
    );

    // Check the event was not processed
    assert!(gh_client.check_runs().is_empty());
}

// Helpers.

/// GHClient fake that records the check runs created (context and head sha).
/// Calling any other method makes the test fail, as no other calls are
/// expected.
#[derive(Clone, Default)]
struct FakeGHClient {
    /// Check runs created so far.
    check_runs: Arc<Mutex<Vec<(Ctx, String)>>>,
    /// Whether creating check runs fails (after recording them).
    create_check_run_fails: bool,
}

impl FakeGHClient {
    /// Create a new FakeGHClient instance.
    fn new() -> Self {
        Self::default()
    }

    /// Get the check runs created so far.
    fn check_runs(&self) -> Vec<(Ctx, String)> {
        self.check_runs.lock().unwrap().clone()
    }

    /// Create a new FakeGHClient instance that fails to create check runs.
    fn failing() -> Self {
        Self {
            create_check_run_fails: true,
            ..Self::default()
        }
    }

    /// Convert the fake into a GHClient trait object.
    fn into_dyn(self) -> DynGHClient {
        Arc::new(self)
    }
}

#[async_trait]
impl GHClient for FakeGHClient {
    async fn add_label(&self, _ctx: &Ctx, _issue_number: i64, _name: &str) -> Result<()> {
        panic!("unexpected call to add_label")
    }

    async fn can_manage_labels(&self, _ctx: &Ctx) -> Result<bool> {
        panic!("unexpected call to can_manage_labels")
    }

    async fn compare_commits(&self, _ctx: &Ctx, _base_sha: &str, _head_sha: &str) -> Result<Vec<Commit>> {
        panic!("unexpected call to compare_commits")
    }

    async fn create_check_run(&self, ctx: &Ctx, check_run: &CheckRun) -> Result<()> {
        // Record check run
        self.check_runs.lock().unwrap().push((ctx.clone(), check_run.head_sha().to_string()));

        // Fail when requested
        if self.create_check_run_fails {
            return Err(format_err!("fake error"));
        }
        Ok(())
    }

    async fn get_config(&self, _ctx: &Ctx) -> Result<Option<Config>> {
        panic!("unexpected call to get_config")
    }

    async fn get_pull_request(&self, _ctx: &Ctx, _number: i64) -> Result<PullRequestState> {
        panic!("unexpected call to get_pull_request")
    }

    async fn is_organization_member(&self, _ctx: &Ctx, _org: &str, _login: &str) -> Result<bool> {
        panic!("unexpected call to is_organization_member")
    }

    async fn remove_label(&self, _ctx: &Ctx, _issue_number: i64, _name: &str) -> Result<()> {
        panic!("unexpected call to remove_label")
    }
}

/// Send the request provided to the router and return the response status
/// and body.
async fn send(router: Router, request: Request<Body>) -> (StatusCode, String) {
    let response = router.oneshot(request).await.unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

/// Compute the signature header value GitHub would send for the body.
fn sign(secret: &str, body: &[u8]) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(body);
    format!("sha256={}", hex::encode(mac.finalize().into_bytes()))
}

/// Build a header map containing the signature provided.
fn signature_headers(signature: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(EVENT_SIGNATURE_HEADER, HeaderValue::from_str(signature).unwrap());
    headers
}

/// Build a webhook request with the event, signature and body provided.
fn webhook_request(event: Option<&str>, signature: Option<&str>, body: &[u8]) -> Request<Body> {
    let mut builder = Request::builder().method(Method::POST).uri("/webhook/github");
    if let Some(event) = event {
        builder = builder.header(EVENT_NAME_HEADER, event);
    }
    if let Some(signature) = signature {
        builder = builder.header(EVENT_SIGNATURE_HEADER, signature);
    }
    builder.body(Body::from(body.to_vec())).unwrap()
}

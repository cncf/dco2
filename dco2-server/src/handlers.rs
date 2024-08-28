//! This module defines the router and handlers used to process HTTP requests.

use anyhow::{format_err, Error, Result};
use axum::{
    body::Bytes,
    extract::{FromRef, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use dco2::{
    dco,
    github::{DynGHClient, Event, EventError, EVENT_ID_HEADER, SIGNATURE_HEADER},
};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::{error, info, instrument};

/// Router's state.
#[derive(Clone, FromRef)]
struct RouterState {
    gh_client: DynGHClient,
    webhook_secret: WebhookSecret,
}

/// Type alias to represent a webhook secret.
pub type WebhookSecret = String;

/// Setup HTTP server router.
pub fn setup_router(gh_client: DynGHClient, webhook_secret: &str) -> Router {
    // Setup router's state
    let state = RouterState {
        gh_client,
        webhook_secret: webhook_secret.to_string(),
    };

    // Setup router
    Router::new()
        .route("/health-check", get(health_check))
        .route("/webhook/github", post(event))
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()))
        .with_state(state)
}

/// Handler that takes care of health check requests.
async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}

/// Handler that processes webhook events from GitHub.
#[instrument(fields(event_id), skip_all, err(Debug))]
async fn event(
    State(gh_client): State<DynGHClient>,
    State(webhook_secret): State<WebhookSecret>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    // Record event_id as part of the current span
    if let Some(event_id) = headers.get(EVENT_ID_HEADER) {
        tracing::Span::current().record("event_id", event_id.to_str().unwrap_or_default());
    }

    // Verify request signature
    if verify_signature(webhook_secret.as_bytes(), &headers, &body).is_err() {
        return Err((StatusCode::BAD_REQUEST, "no valid signature found".to_string()));
    }

    // Parse event from request payload
    let event = match Event::try_from((&headers, &body)) {
        Ok(event) => event,
        Err(err @ (EventError::MissingHeader | EventError::InvalidPayload)) => {
            return Err((StatusCode::BAD_REQUEST, err.to_string()))
        }
        Err(EventError::UnsupportedEvent) => return Ok(()),
    };

    // Process event and run DCO check
    if let Err(err) = dco::process_event(gh_client, &event).await {
        error!(?err, "error processing event");
        return Err((StatusCode::INTERNAL_SERVER_ERROR, String::new()));
    }
    info!("event processed successfully");

    Ok(())
}

/// Verify that the signature provided in the webhook request is valid.
#[allow(clippy::missing_errors_doc)]
pub fn verify_signature(secret: &[u8], headers: &HeaderMap, body: &[u8]) -> Result<()> {
    if let Some(signature) = headers
        .get(SIGNATURE_HEADER)
        .and_then(|s| s.to_str().ok())
        .and_then(|s| s.strip_prefix("sha256="))
        .and_then(|s| hex::decode(s).ok())
    {
        let mut mac = Hmac::<Sha256>::new_from_slice(secret)?;
        mac.update(body.as_ref());
        mac.verify_slice(&signature[..]).map_err(Error::new)
    } else {
        Err(format_err!("no valid signature found"))
    }
}

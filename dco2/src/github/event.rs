//! This module defines some types and functions to parse and deserialize
//! GitHub webhook events.

use super::client::Ctx;
use bytes::Bytes;
use http::HeaderMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Header representing the event unique identifier.
pub const EVENT_ID_HEADER: &str = "X-GitHub-Delivery";

/// Header representing the name of the event received.
pub const EVENT_NAME_HEADER: &str = "X-GitHub-Event";

/// Header representing the event payload signature.
pub const EVENT_SIGNATURE_HEADER: &str = "X-Hub-Signature-256";

/// Webhook event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Event {
    CheckRun(CheckRunEvent),
    PullRequest(PullRequestEvent),
}

impl TryFrom<(&HeaderMap, &Bytes)> for Event {
    type Error = EventError;

    /// Try to create a new event instance from the provided headers and body.
    fn try_from((headers, body): (&HeaderMap, &Bytes)) -> Result<Self, Self::Error> {
        match headers.get(EVENT_NAME_HEADER) {
            Some(event_name) => match event_name.as_bytes() {
                b"check_run" => {
                    let event = serde_json::from_slice(body).map_err(|_| EventError::InvalidPayload)?;
                    Ok(Event::CheckRun(event))
                }
                b"pull_request" => {
                    let event = serde_json::from_slice(body).map_err(|_| EventError::InvalidPayload)?;
                    Ok(Event::PullRequest(event))
                }
                _ => Err(EventError::UnsupportedEvent),
            },
            None => Err(EventError::MissingHeader),
        }
    }
}

/// Errors that may occur while creating a new event instance.
#[derive(Error, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EventError {
    #[error("invalid payload")]
    InvalidPayload,
    #[error("event header missing")]
    MissingHeader,
    #[error("unsupported event")]
    UnsupportedEvent,
}

/// Check run event payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckRunEvent {
    pub action: CheckRunEventAction,
    pub check_run: CheckRunEventCheckRun,
    pub installation: Installation,
    pub repository: Repository,
    pub requested_action: RequestedAction,
}

impl CheckRunEvent {
    /// Get context information from event details.
    pub fn ctx(&self) -> Ctx {
        let (owner, repo) = split_full_name(&self.repository.full_name);
        Ctx {
            inst_id: self.installation.id,
            owner: owner.to_string(),
            repo: repo.to_string(),
        }
    }
}

/// Check run event action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckRunEventAction {
    RequestedAction,
    Rerequested,
}

/// Check run event check run details.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckRunEventCheckRun {
    pub head_sha: String,
}

/// GitHub application installation information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Installation {
    pub id: i64,
}

/// Pull request information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PullRequest {
    pub base: PullRequestBase,
    pub head: PullRequestHead,
    pub html_url: String,
}

/// Pull request base information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PullRequestBase {
    #[serde(rename = "ref")]
    pub ref_: String,
    pub sha: String,
}

/// Pull request event payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PullRequestEvent {
    pub action: PullRequestEventAction,
    pub installation: Installation,
    pub pull_request: PullRequest,
    pub repository: Repository,
}

impl PullRequestEvent {
    /// Get context information from event details.
    pub fn ctx(&self) -> Ctx {
        let (owner, repo) = split_full_name(&self.repository.full_name);
        Ctx {
            inst_id: self.installation.id,
            owner: owner.to_string(),
            repo: repo.to_string(),
        }
    }
}

/// Pull request event action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PullRequestEventAction {
    Opened,
    Synchronize,
    #[serde(other)]
    Other,
}

/// Pull request head information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PullRequestHead {
    #[serde(rename = "ref")]
    pub ref_: String,
    pub sha: String,
}

/// Repository information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Repository {
    pub full_name: String,
}

/// Requested action information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestedAction {
    pub identifier: String,
}

/// Helper function that splits a repository's full name and returns the owner
/// and the repo name as a tuple.
fn split_full_name(full_name: &str) -> (&str, &str) {
    let mut parts = full_name.split('/');
    (parts.next().unwrap(), parts.next().unwrap())
}
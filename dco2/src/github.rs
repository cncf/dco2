//! This module defines an abstraction layer over the GitHub API.

use anyhow::{bail, Result};
use async_trait::async_trait;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use http::HeaderMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

/// Header representing the event unique identifier.
pub const EVENT_ID_HEADER: &str = "X-GitHub-Delivery";

/// Header representing the name of the event received.
pub const EVENT_NAME_HEADER: &str = "X-GitHub-Event";

/// Header representing the event payload signature.
pub const SIGNATURE_HEADER: &str = "X-Hub-Signature-256";

/// Abstraction layer over a GitHub client. This trait defines the methods that
/// a GHClient implementation must provide.
#[async_trait]
pub trait GHClient {
    /// Compare two commits.
    async fn compare_commits(&self, ctx: &Ctx, base_sha: &str, head_sha: &str) -> Result<Vec<Commit>>;

    /// Create a check run.
    async fn create_check_run(&self, ctx: &Ctx, check_run: &CheckRun) -> Result<()>;
}

/// Type alias to represent a GHClient trait object.
pub type DynGHClient = Arc<dyn GHClient + Send + Sync>;

/// GHClient implementation powered by octorust.
#[derive(Clone)]
pub struct GHClientOctorust {
    api_host: Option<String>,
    app_credentials: octorust::auth::JWTCredentials,
}

impl GHClientOctorust {
    /// Create a new GHClientOctorust instance.
    pub fn new(cfg: &AppConfig) -> Result<Self> {
        // Setup credentials
        let private_key = pem::parse(&cfg.private_key)?.contents().to_owned();
        let app_credentials = octorust::auth::JWTCredentials::new(cfg.app_id, private_key)?;

        Ok(Self {
            api_host: cfg.api_host.clone(),
            app_credentials,
        })
    }

    /// Setup a new GitHub client for the installation id provided.
    fn setup_client(&self, inst_id: i64) -> Result<octorust::Client> {
        // Setup credentials
        let user_agent = format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        let tg = octorust::auth::InstallationTokenGenerator::new(inst_id, self.app_credentials.clone());
        let credentials = octorust::auth::Credentials::InstallationToken(tg);

        // Setup client
        let mut client = octorust::Client::new(user_agent, credentials)?;
        if let Some(api_host) = &self.api_host {
            client.with_host_override(api_host);
        }

        Ok(client)
    }
}

#[async_trait]
impl GHClient for GHClientOctorust {
    /// [GHClient::compare_commits]
    async fn compare_commits(&self, ctx: &Ctx, base_sha: &str, head_sha: &str) -> Result<Vec<Commit>> {
        let client = self.setup_client(ctx.inst_id)?;
        let basehead = format!("{}...{}", base_sha, head_sha);
        let commits = client
            .repos()
            .compare_commits(&ctx.owner, &ctx.repo, 0, 0, &basehead)
            .await?
            .body
            .commits
            .into_iter()
            .map(Into::into)
            .collect();

        Ok(commits)
    }

    /// [GHClient::create_check_run]
    async fn create_check_run(&self, ctx: &Ctx, check_run: &CheckRun) -> Result<()> {
        let client = self.setup_client(ctx.inst_id)?;
        let conclusion = match check_run.conclusion.as_str() {
            "success" => octorust::types::ChecksCreateRequestConclusion::Success,
            "failure" => octorust::types::ChecksCreateRequestConclusion::Failure,
            _ => bail!("invalid conclusion: {}", check_run.conclusion),
        };
        let status = match check_run.status.as_str() {
            "completed" => octorust::types::JobStatus::Completed,
            _ => bail!("invalid status: {}", check_run.status),
        };
        let body = octorust::types::ChecksCreateRequest {
            actions: vec![],
            completed_at: Some(Utc::now()),
            conclusion: Some(conclusion),
            details_url: String::new(),
            external_id: String::new(),
            head_sha: check_run.head_sha.clone(),
            name: check_run.name.clone(),
            output: Some(octorust::types::ChecksCreateRequestOutput {
                annotations: vec![],
                images: vec![],
                summary: check_run.summary.clone(),
                text: String::new(),
                title: check_run.name.clone(),
            }),
            started_at: Some(check_run.started_at),
            status: Some(status),
        };
        client.checks().create(&ctx.owner, &ctx.repo, &body).await?;

        Ok(())
    }
}

/// GitHub application configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AppConfig {
    pub api_host: Option<String>,
    pub app_id: i64,
    pub private_key: String,
    pub webhook_secret: String,
}

/// Check run information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckRun {
    pub conclusion: String,
    pub head_sha: String,
    pub name: String,
    pub started_at: DateTime<Utc>,
    pub status: String,
    pub summary: String,
}

/// Commit information.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Commit {
    pub author: Option<GitUser>,
    pub committer: Option<GitUser>,
    pub html_url: String,
    pub is_merge: bool,
    pub message: String,
    pub sha: String,
}

/// Git user information.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GitUser {
    pub name: String,
    pub email: String,
    pub is_bot: bool,
}

impl From<octorust::types::CommitDataType> for Commit {
    /// Convert octorust commit data to Commit.
    fn from(c: octorust::types::CommitDataType) -> Self {
        Self {
            author: c.commit.author.map(|author| GitUser {
                name: author.name,
                email: author.email,
                is_bot: c.author.map_or(false, |a| a.type_ == "Bot"),
            }),
            committer: c.commit.committer.map(|committer| GitUser {
                name: committer.name,
                email: committer.email,
                is_bot: c.committer.map_or(false, |c| c.type_ == "Bot"),
            }),
            html_url: c.html_url,
            is_merge: c.parents.len() > 1,
            message: c.commit.message,
            sha: c.sha,
        }
    }
}

/// Information about the target of a GitHub API request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ctx {
    pub inst_id: i64,
    pub owner: String,
    pub repo: String,
}

/// Webhook event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Event {
    PullRequest(PullRequestEvent),
}

impl Event {
    /// Get context information from event details.
    pub fn ctx(&self) -> Ctx {
        match self {
            Event::PullRequest(event) => {
                let (owner, repo) = split_full_name(&event.repository.full_name);
                Ctx {
                    inst_id: event.installation.id,
                    owner: owner.to_string(),
                    repo: repo.to_string(),
                }
            }
        }
    }

    /// Get the installation id associated with the event.
    pub fn installation_id(&self) -> i64 {
        match self {
            Event::PullRequest(event) => event.installation.id,
        }
    }
}

impl TryFrom<(&HeaderMap, &Bytes)> for Event {
    type Error = EventError;

    /// Try to create a new event instance from the provided headers and body.
    fn try_from((headers, body): (&HeaderMap, &Bytes)) -> Result<Self, Self::Error> {
        match headers.get(EVENT_NAME_HEADER) {
            Some(event_name) => match event_name.as_bytes() {
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

/// Helper function that splits a repository's full name and returns the owner
/// and the repo name as a tuple.
fn split_full_name(full_name: &str) -> (&str, &str) {
    let mut parts = full_name.split('/');
    (parts.next().unwrap(), parts.next().unwrap())
}

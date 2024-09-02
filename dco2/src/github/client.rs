//! This module defines an abstraction layer over the GitHub API.

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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

        let body = octorust::types::ChecksCreateRequest {
            actions: check_run.actions.iter().cloned().map(Into::into).collect(),
            completed_at: Some(check_run.completed_at),
            conclusion: Some(check_run.conclusion.clone().into()),
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
            status: Some(check_run.status.clone().into()),
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
    pub actions: Vec<CheckRunAction>,
    pub completed_at: DateTime<Utc>,
    pub conclusion: CheckRunConclusion,
    pub head_sha: String,
    pub name: String,
    pub started_at: DateTime<Utc>,
    pub status: CheckRunStatus,
    pub summary: String,
}

/// Check run action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckRunAction {
    pub label: String,
    pub description: String,
    pub identifier: String,
}

impl From<CheckRunAction> for octorust::types::ChecksCreateRequestActions {
    /// Convert CheckRunAction to octorust ChecksCreateRequestActions.
    fn from(a: CheckRunAction) -> octorust::types::ChecksCreateRequestActions {
        octorust::types::ChecksCreateRequestActions {
            label: a.label,
            description: a.description,
            identifier: a.identifier,
        }
    }
}

/// Check run conclusion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckRunConclusion {
    Success,
    ActionRequired,
}

impl From<CheckRunConclusion> for octorust::types::ChecksCreateRequestConclusion {
    /// Convert CheckRunConclusion to octorust ChecksCreateRequestConclusion.
    fn from(c: CheckRunConclusion) -> octorust::types::ChecksCreateRequestConclusion {
        match c {
            CheckRunConclusion::Success => octorust::types::ChecksCreateRequestConclusion::Success,
            CheckRunConclusion::ActionRequired => {
                octorust::types::ChecksCreateRequestConclusion::ActionRequired
            }
        }
    }
}

/// Check run status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckRunStatus {
    Completed,
}

impl From<CheckRunStatus> for octorust::types::JobStatus {
    /// Convert CheckRunStatus to octorust JobStatus.
    fn from(s: CheckRunStatus) -> octorust::types::JobStatus {
        match s {
            CheckRunStatus::Completed => octorust::types::JobStatus::Completed,
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

/// Git user information.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GitUser {
    pub name: String,
    pub email: String,
    pub is_bot: bool,
}

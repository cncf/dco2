//! This module defines an abstraction layer over the GitHub API.

use anyhow::{bail, Result};
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as b64, Engine as _};
use cached::proc_macro::cached;
use chrono::{DateTime, Utc};
use http::StatusCode;
#[cfg(test)]
use mockall::automock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::warn;

/// Path of the configuration file in the repository.
const CONFIG_FILE_PATH: &str = ".github/dco.yml";

/// Abstraction layer over a GitHub client. This trait defines the methods that
/// a GHClient implementation must provide.
#[async_trait]
#[cfg_attr(test, automock)]
pub trait GHClient {
    /// Compare two commits.
    async fn compare_commits(&self, ctx: &Ctx, base_sha: &str, head_sha: &str) -> Result<Vec<Commit>>;

    /// Create a check run.
    async fn create_check_run(&self, ctx: &Ctx, check_run: &CheckRun) -> Result<()>;

    /// Get configuration.
    async fn get_config(&self, ctx: &Ctx) -> Result<Option<Config>>;

    /// Check if a user is a member of the organization.
    async fn is_organization_member(&self, ctx: &Ctx, org: &str, login: &str) -> Result<bool>;
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
        // Setup client for installation provided
        let client = self.setup_client(ctx.inst_id)?;

        // Compare commits
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
        // Setup client for installation provided
        let client = self.setup_client(ctx.inst_id)?;

        // Create check run
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

    /// [GHClient::get_config]
    async fn get_config(&self, ctx: &Ctx) -> Result<Option<Config>> {
        // Setup client for installation provided
        let client = self.setup_client(ctx.inst_id)?;

        // Get configuration file content
        let resp = match client.repos().get_content_file(&ctx.owner, &ctx.repo, CONFIG_FILE_PATH, "").await {
            Ok(resp) => resp,
            Err(octorust::ClientError::HttpError {
                status,
                headers: _,
                error,
            }) => {
                if status == StatusCode::NOT_FOUND {
                    return Ok(None);
                }
                bail!(error);
            }
            Err(err) => bail!(err),
        };

        // Decode and parse configuration
        let mut b64data = resp.body.content.as_bytes().to_owned();
        b64data.retain(|b| !b" \n\t\r\x0b\x0c".contains(b));
        let data = String::from_utf8(b64.decode(b64data)?)?;
        let config = serde_yaml::from_str(&data)?;

        Ok(config)
    }

    /// [GHClient::is_organization_member]
    async fn is_organization_member(&self, ctx: &Ctx, org: &str, username: &str) -> Result<bool> {
        #[cached(
            time = 3600,
            sync_writes = true,
            result = true,
            key = "String",
            convert = r#"{ format!("{}-{}", org, username) }"#
        )]
        async fn inner(client: &octorust::Client, org: &str, username: &str) -> Result<bool> {
            // Check if user is a member of the organization
            let resp = client.orgs().check_membership_for_user(org, username).await?;
            Ok(resp.status == StatusCode::NO_CONTENT)
        }

        // Setup client for installation provided
        let client = self.setup_client(ctx.inst_id)?;

        inner(&client, org, username).await
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
    actions: Vec<CheckRunAction>,
    completed_at: DateTime<Utc>,
    conclusion: CheckRunConclusion,
    head_sha: String,
    name: String,
    started_at: DateTime<Utc>,
    status: CheckRunStatus,
    summary: String,
}

impl CheckRun {
    /// Create a new CheckRun instance.
    pub fn new(input: NewCheckRunInput) -> Self {
        // Create a new check run from the input received.
        let mut check_run = Self {
            actions: input.actions,
            completed_at: input.completed_at,
            conclusion: input.conclusion,
            head_sha: input.head_sha,
            name: input.name,
            started_at: input.started_at,
            status: input.status,
            summary: input.summary,
        };

        // Make sure the length of some fields is below the maximum allowed by
        // GitHub (we'll truncate them if necessary).

        // Output summary
        const MAX_OUTPUT_SUMMARY_LENGTH: usize = 65535;
        if check_run.summary.len() > MAX_OUTPUT_SUMMARY_LENGTH {
            check_run.summary.truncate(MAX_OUTPUT_SUMMARY_LENGTH);
            warn!("check run summary truncated");
        }

        // Actions
        for action in &mut check_run.actions {
            // Action label
            const MAX_ACTION_LABEL_LENGTH: usize = 20;
            if action.label.len() > MAX_ACTION_LABEL_LENGTH {
                action.label.truncate(MAX_ACTION_LABEL_LENGTH);
                warn!("check run action label truncated");
            }

            // Action description
            const MAX_ACTION_DESCRIPTION_LENGTH: usize = 40;
            if action.description.len() > MAX_ACTION_DESCRIPTION_LENGTH {
                action.description.truncate(MAX_ACTION_DESCRIPTION_LENGTH);
                warn!("check run action description truncated");
            }

            // Action identifier
            const MAX_ACTION_IDENTIFIER_LENGTH: usize = 20;
            if action.identifier.len() > MAX_ACTION_IDENTIFIER_LENGTH {
                action.identifier.truncate(MAX_ACTION_IDENTIFIER_LENGTH);
                warn!("check run action identifier truncated");
            }
        }

        check_run
    }

    /// Get the actions of the check run.
    pub fn actions(&self) -> &[CheckRunAction] {
        &self.actions
    }

    /// Get the completion time of the check run.
    pub fn completed_at(&self) -> &DateTime<Utc> {
        &self.completed_at
    }

    /// Get the conclusion of the check run.
    pub fn conclusion(&self) -> &CheckRunConclusion {
        &self.conclusion
    }

    /// Get the head SHA of the check run.
    pub fn head_sha(&self) -> &str {
        &self.head_sha
    }

    /// Get the name of the check run.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the start time of the check run.
    pub fn started_at(&self) -> &DateTime<Utc> {
        &self.started_at
    }

    /// Get the status of the check run.
    pub fn status(&self) -> &CheckRunStatus {
        &self.status
    }

    /// Get the summary of the check run.
    pub fn summary(&self) -> &str {
        &self.summary
    }
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
    pub author: Option<User>,
    pub committer: Option<User>,
    pub html_url: String,
    pub is_merge: bool,
    pub message: String,
    pub sha: String,
    pub verified: Option<bool>,
}

impl From<octorust::types::CommitDataType> for Commit {
    /// Convert octorust commit data to Commit.
    fn from(c: octorust::types::CommitDataType) -> Self {
        Self {
            author: c.commit.author.map(|author| User {
                name: author.name,
                email: author.email,
                is_bot: c.author.as_ref().map_or(false, |a| a.type_ == "Bot"),
                login: c.author.map(|a| a.login),
            }),
            committer: c.commit.committer.map(|committer| User {
                name: committer.name,
                email: committer.email,
                is_bot: c.committer.as_ref().map_or(false, |c| c.type_ == "Bot"),
                login: c.committer.map(|c| c.login),
            }),
            html_url: c.html_url,
            is_merge: c.parents.len() > 1,
            message: c.commit.message,
            sha: c.sha,
            verified: c.commit.verification.map(|v| v.verified),
        }
    }
}

/// Default values for the configuration.
pub const DEFAULT_INDIVIDUAL_REMEDIATION_COMMITS_ALLOWED: bool = false;
pub const DEFAULT_THIRD_PARTY_REMEDIATION_COMMITS_ALLOWED: bool = false;
pub const DEFAULT_MEMBERS_SIGNOFF_REQUIRED: bool = true;

/// Repository configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct Config {
    pub allow_remediation_commits: Option<ConfigAllowRemediationCommits>,
    pub require: Option<ConfigRequire>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            allow_remediation_commits: Some(ConfigAllowRemediationCommits::default()),
            require: Some(ConfigRequire::default()),
        }
    }
}

impl Config {
    /// Check if individual remediation commits are allowed.
    pub fn individual_remediation_commits_are_allowed(&self) -> bool {
        if let Some(allow_remediation_commits) = &self.allow_remediation_commits {
            allow_remediation_commits
                .individual
                .unwrap_or(DEFAULT_INDIVIDUAL_REMEDIATION_COMMITS_ALLOWED)
        } else {
            DEFAULT_INDIVIDUAL_REMEDIATION_COMMITS_ALLOWED
        }
    }

    /// Check if third party remediation commits are allowed.
    pub fn third_party_remediation_commits_are_allowed(&self) -> bool {
        if let Some(allow_remediation_commits) = &self.allow_remediation_commits {
            allow_remediation_commits
                .third_party
                .unwrap_or(DEFAULT_THIRD_PARTY_REMEDIATION_COMMITS_ALLOWED)
        } else {
            DEFAULT_THIRD_PARTY_REMEDIATION_COMMITS_ALLOWED
        }
    }

    /// Check if the configuration requires members to sign-off commits.
    pub fn members_signoff_is_required(&self) -> bool {
        if let Some(require) = &self.require {
            require.members.unwrap_or(DEFAULT_MEMBERS_SIGNOFF_REQUIRED)
        } else {
            DEFAULT_MEMBERS_SIGNOFF_REQUIRED
        }
    }
}

/// Allow remediation commits section of the configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct ConfigAllowRemediationCommits {
    /// Indicates whether individual remediation commits are allowed or not.
    /// (default: false)
    pub individual: Option<bool>,

    /// Indicates whether third party remediation commits are allowed or not.
    /// (default: false)
    pub third_party: Option<bool>,
}

impl Default for ConfigAllowRemediationCommits {
    fn default() -> Self {
        Self {
            individual: Some(DEFAULT_INDIVIDUAL_REMEDIATION_COMMITS_ALLOWED),
            third_party: Some(DEFAULT_THIRD_PARTY_REMEDIATION_COMMITS_ALLOWED),
        }
    }
}

/// Require section of the configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct ConfigRequire {
    /// Indicates whether members are required to sign-off or not.
    /// (default: true)
    pub members: Option<bool>,
}

impl Default for ConfigRequire {
    fn default() -> Self {
        Self {
            members: Some(DEFAULT_MEMBERS_SIGNOFF_REQUIRED),
        }
    }
}

/// User information.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub name: String,
    pub email: String,
    pub is_bot: bool,
    pub login: Option<String>,
}

impl User {
    /// Check if the user matches the provided user (if any).
    pub fn matches(&self, user: &Option<User>) -> bool {
        if let Some(user) = user {
            self.name.to_lowercase() == user.name.to_lowercase()
                && self.email.to_lowercase() == user.email.to_lowercase()
        } else {
            false
        }
    }
}

/// Input used to create a new check run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewCheckRunInput {
    pub actions: Vec<CheckRunAction>,
    pub completed_at: DateTime<Utc>,
    pub conclusion: CheckRunConclusion,
    pub head_sha: String,
    pub name: String,
    pub started_at: DateTime<Utc>,
    pub status: CheckRunStatus,
    pub summary: String,
}

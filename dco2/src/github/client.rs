//! This module defines an abstraction layer over the GitHub API.

use std::{
    sync::{Arc, Mutex, PoisonError},
    time::Duration,
};

use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::STANDARD as b64};
use cached::{Cached, LruCache, LruTtlCache, cached};
use chrono::{DateTime, Utc};
use http::{StatusCode, header::RETRY_AFTER};
#[cfg(test)]
use mockall::automock;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, warn};

/// Path of the configuration file in the repository.
const CONFIG_FILE_PATH: &str = ".github/dco.yml";

/// Color used when creating the label applied to pull requests.
const LABEL_COLOR: &str = "ededed";

/// Description used when creating the label applied to pull requests.
const LABEL_DESCRIPTION: &str = "DCO check requires attention";

/// Time during which the labels permissions of an installation are cached.
const LABELS_PERMISSIONS_TTL: Duration = Duration::from_secs(3600);

/// Maximum number of installation clients kept in the clients cache.
const MAX_CACHED_CLIENTS: usize = 1000;

/// Maximum number of installations kept in the labels permissions cache.
const MAX_CACHED_LABELS_PERMISSIONS: usize = 1000;

/// Message returned by GitHub when the app lacks the permissions required.
const PERMISSIONS_NOT_GRANTED_MESSAGE: &str = "Resource not accessible by integration";

/// Header used by GitHub to report the requests remaining in the rate limit.
const RATE_LIMIT_REMAINING_HEADER: &str = "x-ratelimit-remaining";

/// User agent used in requests to the GitHub API.
const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

/// Abstraction layer over a GitHub client. This trait defines the methods that
/// a GHClient implementation must provide.
#[async_trait]
#[cfg_attr(test, automock)]
pub trait GHClient {
    /// Add a label to an issue or pull request, creating it in the repository
    /// when it does not exist.
    async fn add_label(&self, ctx: &Ctx, issue_number: i64, name: &str) -> Result<()>;

    /// Check if the installation has been granted the permissions required to
    /// manage labels.
    async fn can_manage_labels(&self, ctx: &Ctx) -> Result<bool>;

    /// Compare two commits.
    async fn compare_commits(&self, ctx: &Ctx, base_sha: &str, head_sha: &str) -> Result<Vec<Commit>>;

    /// Create a check run.
    async fn create_check_run(&self, ctx: &Ctx, check_run: &CheckRun) -> Result<()>;

    /// Get configuration.
    async fn get_config(&self, ctx: &Ctx) -> Result<Option<Config>>;

    /// Get the current state of a pull request.
    async fn get_pull_request(&self, ctx: &Ctx, number: i64) -> Result<PullRequestState>;

    /// Check if a user is a member of the organization.
    async fn is_organization_member(&self, ctx: &Ctx, org: &str, login: &str) -> Result<bool>;

    /// Remove a label from an issue or pull request. Removing a label that is
    /// not applied is not an error.
    async fn remove_label(&self, ctx: &Ctx, issue_number: i64, name: &str) -> Result<()>;
}

/// Type alias to represent a GHClient trait object.
pub type DynGHClient = Arc<dyn GHClient + Send + Sync>;

/// GHClient implementation powered by octorust.
#[derive(Clone)]
pub struct GHClientOctorust {
    /// Client authenticated as the GitHub App (used for app level endpoints).
    app_client: octorust::Client,
    /// GitHub App credentials used to request installation tokens.
    app_credentials: octorust::auth::JWTCredentials,
    /// Clients cache, keyed by installation id.
    clients: Arc<Mutex<LruCache<i64, octorust::Client>>>,
    /// Labels permissions cache, keyed by installation id.
    labels_permissions: Arc<Mutex<LruTtlCache<i64, bool>>>,

    /// GitHub API host override (e.g. for GitHub Enterprise Server).
    api_host: Option<String>,
}

impl GHClientOctorust {
    /// Create a new GHClientOctorust instance.
    pub fn new(cfg: &AppConfig) -> Result<Self> {
        // Setup credentials
        let private_key = pem::parse(&cfg.private_key)?.contents().to_owned();
        let app_credentials = octorust::auth::JWTCredentials::new(cfg.app_id, private_key)?;

        // Setup app client
        let credentials = octorust::auth::Credentials::JWT(app_credentials.clone());
        let mut app_client = octorust::Client::new(USER_AGENT, credentials)?;
        if let Some(api_host) = &cfg.api_host {
            app_client.with_host_override(api_host);
        }

        // Setup caches
        let clients = LruCache::builder().max_size(MAX_CACHED_CLIENTS).build()?;
        let labels_permissions = LruTtlCache::builder()
            .max_size(MAX_CACHED_LABELS_PERMISSIONS)
            .ttl(LABELS_PERMISSIONS_TTL)
            .build()?;

        Ok(Self {
            app_client,
            app_credentials,
            clients: Arc::new(Mutex::new(clients)),
            labels_permissions: Arc::new(Mutex::new(labels_permissions)),

            api_host: cfg.api_host.clone(),
        })
    }

    /// Create the label provided in the repository. The error returned when
    /// the label already exists (e.g. created concurrently) is ignored.
    async fn create_label(&self, client: &octorust::Client, ctx: &Ctx, name: &str) -> Result<()> {
        // Prepare label details
        let body = octorust::types::IssuesCreateLabelRequest {
            color: LABEL_COLOR.to_string(),
            description: LABEL_DESCRIPTION.to_string(),
            name: name.to_string(),
        };

        // Create label, ignoring the error returned when it already exists
        match client.issues().create_label(&ctx.owner, &ctx.repo, &body).await {
            Ok(_) => Ok(()),
            Err(octorust::ClientError::HttpError { status, .. })
                if status == StatusCode::UNPROCESSABLE_ENTITY =>
            {
                Ok(())
            }
            Err(err) => Err(self.labels_error(ctx.inst_id, err)),
        }
    }

    /// Create the label provided in the repository when it does not exist.
    /// Existing labels are never modified.
    async fn ensure_label(&self, client: &octorust::Client, ctx: &Ctx, name: &str) -> Result<()> {
        // Check if the label exists, creating it when it's not found
        let encoded_name = encode_label_name(name);
        match client.issues().get_label(&ctx.owner, &ctx.repo, &encoded_name).await {
            Ok(_) => Ok(()),
            Err(octorust::ClientError::HttpError { status, .. }) if status == StatusCode::NOT_FOUND => {
                self.create_label(client, ctx, name).await.context("error creating label")
            }
            Err(err) => Err(self.labels_error(ctx.inst_id, err)).context("error getting label"),
        }
    }

    /// Remove the installation's client and labels permissions from their
    /// caches, so that a new client (with a new access token) is set up and
    /// the permissions are checked again on the next request.
    fn evict_installation(&self, inst_id: i64) {
        // Evict client
        let mut clients = self.clients.lock().unwrap_or_else(PoisonError::into_inner);
        clients.cache_remove(&inst_id);
        drop(clients);

        // Evict labels permissions
        let mut labels_permissions = self.labels_permissions.lock().unwrap_or_else(PoisonError::into_inner);
        labels_permissions.cache_remove(&inst_id);
    }

    /// Convert an error returned by a labels endpoint. When GitHub reports
    /// that the app lacks the permissions required, the installation is
    /// evicted from the caches, as its access token may predate their approval
    /// or the cached permissions may no longer be granted. Other errors are
    /// returned unchanged.
    fn labels_error(&self, inst_id: i64, err: octorust::ClientError) -> anyhow::Error {
        if is_permissions_error(&err) {
            self.evict_installation(inst_id);
            return anyhow::Error::new(LabelsPermissionError);
        }
        err.into()
    }

    /// Returns the GitHub client for the installation id provided.
    ///
    /// Clients are cached per installation so that their access token and HTTP
    /// connections are reused across requests. octorust refreshes the token
    /// automatically before it expires.
    fn setup_client(&self, inst_id: i64) -> Result<octorust::Client> {
        // Reuse the installation's client when available (the cache remains
        // consistent even if a previous holder of the lock panicked)
        let mut clients = self.clients.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(client) = clients.cache_get(&inst_id) {
            return Ok(client.clone());
        }

        // Setup credentials
        let tg = octorust::auth::InstallationTokenGenerator::new(inst_id, self.app_credentials.clone());
        let credentials = octorust::auth::Credentials::InstallationToken(tg);

        // Setup client
        let mut client = octorust::Client::new(USER_AGENT, credentials)?;
        if let Some(api_host) = &self.api_host {
            client.with_host_override(api_host);
        }

        // Cache client for subsequent requests
        clients.cache_set(inst_id, client.clone());

        Ok(client)
    }
}

#[async_trait]
impl GHClient for GHClientOctorust {
    /// [GHClient::add_label]
    async fn add_label(&self, ctx: &Ctx, issue_number: i64, name: &str) -> Result<()> {
        // Setup client for installation provided
        let client = self.setup_client(ctx.inst_id)?;

        // Make sure the label exists, so that it's created with the color and
        // description used by DCO2
        self.ensure_label(&client, ctx, name).await?;

        // Add label to the issue
        let body = octorust::types::IssuesAddLabelsRequestOneOf::IssuesAddLabelsRequest(
            octorust::types::IssuesAddLabelsRequest {
                labels: vec![name.to_string()],
            },
        );
        if let Err(err) = client.issues().add_labels(&ctx.owner, &ctx.repo, issue_number, &body).await {
            return Err(self.labels_error(ctx.inst_id, err));
        }

        Ok(())
    }

    /// [GHClient::can_manage_labels]
    async fn can_manage_labels(&self, ctx: &Ctx) -> Result<bool> {
        // Reuse the installation's cached permissions when available
        let cached_granted = self
            .labels_permissions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .cache_get(&ctx.inst_id)
            .copied();
        if let Some(granted) = cached_granted {
            return Ok(granted);
        }

        // Get installation details (a response that cannot be parsed is
        // handled as if the permissions were not granted)
        let granted = match self.app_client.apps().get_installation(ctx.inst_id).await {
            Ok(resp) => labels_permissions_granted(&resp.body.permissions),
            Err(octorust::ClientError::SerdeJsonError(err)) => {
                warn!(?err, inst_id = ctx.inst_id, "error parsing installation details");
                false
            }
            Err(err) => bail!(err),
        };

        // Cache permissions for subsequent requests
        self.labels_permissions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .cache_set(ctx.inst_id, granted);

        Ok(granted)
    }

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
            external_id: check_run.external_id.clone().unwrap_or_default(),
            head_sha: check_run.head_sha.clone(),
            name: check_run.name.clone(),
            output: Some(octorust::types::ChecksCreateRequestOutput {
                annotations: vec![],
                images: vec![],
                summary: check_run.summary.clone(),
                text: String::new(),
                title: check_run.title.clone(),
            }),
            started_at: Some(check_run.started_at),
            status: Some(check_run.status.clone().into()),
        };
        client.checks().create(&ctx.owner, &ctx.repo, &body).await?;

        Ok(())
    }

    /// [GHClient::get_config]
    async fn get_config(&self, ctx: &Ctx) -> Result<Option<Config>> {
        #[cached(
            max_size = 1000,
            ttl = 3600,
            sync_writes = "default",
            key = "String",
            convert = r#"{ format!("{}-{}", ctx.owner, ctx.repo) }"#
        )]
        async fn inner(client: &octorust::Client, ctx: &Ctx) -> Result<Option<Config>> {
            // Get configuration file content
            let resp =
                match client.repos().get_content_file(&ctx.owner, &ctx.repo, CONFIG_FILE_PATH, "").await {
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

        // Setup client for installation provided
        let client = self.setup_client(ctx.inst_id)?;

        inner(&client, ctx).await
    }

    /// [GHClient::get_pull_request]
    async fn get_pull_request(&self, ctx: &Ctx, number: i64) -> Result<PullRequestState> {
        // Setup client for installation provided
        let client = self.setup_client(ctx.inst_id)?;

        // Get pull request
        let pr = client.pulls().get(&ctx.owner, &ctx.repo, number).await?.body;

        Ok(PullRequestState {
            head_sha: pr.head.sha,
            is_open: pr.state == octorust::types::State::Open,
            labels: pr.labels.into_iter().map(|label| label.name).collect(),
        })
    }

    /// [GHClient::is_organization_member]
    async fn is_organization_member(&self, ctx: &Ctx, org: &str, username: &str) -> Result<bool> {
        #[cached(
            max_size = 1000,
            ttl = 3600,
            sync_writes = "default",
            key = "String",
            convert = r#"{ format!("{}-{}", org, username) }"#
        )]
        async fn inner(client: &octorust::Client, org: &str, username: &str) -> Result<bool> {
            // Check if user is a member of the organization
            let resp = match client.orgs().check_membership_for_user(org, username).await {
                Ok(resp) => resp,
                Err(octorust::ClientError::HttpError {
                    status,
                    headers: _,
                    error,
                }) => {
                    if status == StatusCode::NOT_FOUND {
                        return Ok(false);
                    }
                    bail!(error);
                }
                Err(err) => bail!(err),
            };
            Ok(resp.status == StatusCode::NO_CONTENT)
        }

        // Setup client for installation provided
        let client = self.setup_client(ctx.inst_id)?;

        inner(&client, org, username).await
    }

    /// [GHClient::remove_label]
    async fn remove_label(&self, ctx: &Ctx, issue_number: i64, name: &str) -> Result<()> {
        // Setup client for installation provided
        let client = self.setup_client(ctx.inst_id)?;

        // Remove label from the issue
        let encoded_name = encode_label_name(name);
        match client.issues().remove_label(&ctx.owner, &ctx.repo, issue_number, &encoded_name).await {
            Ok(_) => Ok(()),
            Err(octorust::ClientError::HttpError { status, .. }) if status == StatusCode::NOT_FOUND => {
                debug!(owner = %ctx.owner, repo = %ctx.repo, issue_number, name, "label to remove not found");
                Ok(())
            }
            Err(err) => Err(self.labels_error(ctx.inst_id, err)),
        }
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
    external_id: Option<String>,
    head_sha: String,
    name: String,
    started_at: DateTime<Utc>,
    status: CheckRunStatus,
    summary: String,
    title: String,
}

impl CheckRun {
    /// Create a new CheckRun instance.
    pub fn new(input: NewCheckRunInput) -> Self {
        // Create a new check run from the input received.
        let mut check_run = Self {
            actions: input.actions,
            completed_at: input.completed_at,
            conclusion: input.conclusion,
            external_id: input.external_id,
            head_sha: input.head_sha,
            name: input.name,
            started_at: input.started_at,
            status: input.status,
            summary: input.summary,
            title: input.title,
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

    /// Get the external id of the check run (the pull request number, when
    /// the check run was created for a pull request).
    pub fn external_id(&self) -> Option<&str> {
        self.external_id.as_deref()
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

    /// Get the title of the check run.
    pub fn title(&self) -> &str {
        &self.title
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
                is_bot: c.author.as_ref().is_some_and(|a| a.type_ == "Bot"),
                login: c.author.map(|a| a.login),
            }),
            committer: c.commit.committer.map(|committer| User {
                name: committer.name,
                email: committer.email,
                is_bot: c.committer.as_ref().is_some_and(|c| c.type_ == "Bot"),
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
pub const DEFAULT_OVERRIDE_ACTION_ALLOWED: bool = true;
pub const DEFAULT_MEMBERS_SIGNOFF_REQUIRED: bool = true;
pub const DEFAULT_INDIVIDUAL_REMEDIATION_COMMITS_ALLOWED: bool = false;
pub const DEFAULT_THIRD_PARTY_REMEDIATION_COMMITS_ALLOWED: bool = false;

/// Repository configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct Config {
    pub allow_override_action: Option<bool>,
    pub allow_remediation_commits: Option<ConfigAllowRemediationCommits>,
    pub label: Option<String>,
    pub require: Option<ConfigRequire>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            allow_override_action: Some(DEFAULT_OVERRIDE_ACTION_ALLOWED),
            allow_remediation_commits: Some(ConfigAllowRemediationCommits::default()),
            label: None,
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

    /// Get the label to apply to pull requests whose DCO check requires
    /// attention. Returns `None` when labeling is disabled or the configured
    /// name is not valid (empty, longer than 50 characters, or a dot segment,
    /// which can't be used in the path of label requests).
    pub fn label(&self) -> Option<&str> {
        const MAX_LABEL_LENGTH: usize = 50;

        let label = self.label.as_deref()?.trim();
        if label.is_empty() || label.chars().count() > MAX_LABEL_LENGTH || label == "." || label == ".." {
            return None;
        }
        Some(label)
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

    /// Check if the override action is allowed.
    pub fn override_action_is_allowed(&self) -> bool {
        self.allow_override_action.unwrap_or(DEFAULT_OVERRIDE_ACTION_ALLOWED)
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

/// Error returned when the app lacks the permissions required to manage
/// labels (e.g. the installation owner hasn't approved them yet).
#[derive(Error, Debug, Clone, PartialEq)]
#[error("permissions to manage labels not granted")]
pub struct LabelsPermissionError;

/// Input used to create a new check run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewCheckRunInput {
    pub actions: Vec<CheckRunAction>,
    pub completed_at: DateTime<Utc>,
    pub conclusion: CheckRunConclusion,
    pub external_id: Option<String>,
    pub head_sha: String,
    pub name: String,
    pub started_at: DateTime<Utc>,
    pub status: CheckRunStatus,
    pub summary: String,
    pub title: String,
}

/// Current state of a pull request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PullRequestState {
    pub head_sha: String,
    pub is_open: bool,
    pub labels: Vec<String>,
}

/// Check if the error provided was caused by GitHub rate limiting the requests
/// (primary or secondary rate limits).
pub fn is_rate_limit_error(err: &anyhow::Error) -> bool {
    err.chain()
        .filter_map(|cause| cause.downcast_ref::<octorust::ClientError>())
        .any(|err| match err {
            octorust::ClientError::RateLimited { .. } => true,
            octorust::ClientError::HttpError {
                status,
                headers,
                error,
            } => {
                *status == StatusCode::TOO_MANY_REQUESTS
                    || (*status == StatusCode::FORBIDDEN
                        && (headers.contains_key(RETRY_AFTER)
                            || headers.get(RATE_LIMIT_REMAINING_HEADER).is_some_and(|v| v == "0")
                            || error.to_lowercase().contains("rate limit")))
            }
            _ => false,
        })
}

/// Encode the characters of a label name that octorust does not encode when
/// building path segments and that would change the request path (`%`, `/`
/// and `\`). Dot segments (`.` and `..`) are rejected by `Config::label`.
fn encode_label_name(name: &str) -> String {
    name.replace('%', "%25").replace('/', "%2F").replace('\\', "%5C")
}

/// Check if the error provided was returned because the app lacks the
/// permissions required. Other 403 responses (e.g. secondary rate limits) are
/// not considered permissions errors.
fn is_permissions_error(err: &octorust::ClientError) -> bool {
    matches!(
        err,
        octorust::ClientError::HttpError { status, error, .. }
            if *status == StatusCode::FORBIDDEN && error.contains(PERMISSIONS_NOT_GRANTED_MESSAGE)
    )
}

/// Check if the permissions provided allow managing labels (write access to
/// pull requests or issues).
fn labels_permissions_granted(permissions: &octorust::types::AppPermissions) -> bool {
    let is_write = |p: &Option<octorust::types::Pages>| matches!(p, Some(octorust::types::Pages::Write));
    is_write(&permissions.pull_requests) || is_write(&permissions.issues)
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn app_permissions(json: &str) -> octorust::types::AppPermissions {
        serde_json::from_str(json).unwrap()
    }

    fn config_with_label(label: &str) -> Config {
        Config {
            label: Some(label.to_string()),
            ..Default::default()
        }
    }

    fn http_error(status: StatusCode, error: &str) -> octorust::ClientError {
        octorust::ClientError::HttpError {
            status,
            headers: http::HeaderMap::new(),
            error: error.to_string(),
        }
    }
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
}

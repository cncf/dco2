//! This module defines some types to represent the server configuration.

use std::path::PathBuf;

use anyhow::{Context, Result};
use figment::{
    Figment,
    providers::{Env, Format, Serialized, Yaml},
};
use serde::{Deserialize, Serialize};

use dco2::github::AppConfig;
use dco2_server::env::env_with_raw_keys;

#[cfg(test)]
mod tests;

/// Configuration keys whose environment variables values must not be parsed.
const RAW_ENV_KEYS: &[&str] = &["github_app.webhook_secret"];

/// Server configuration.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct Config {
    pub github_app: AppConfig,
    pub log_format: LogFormat,
    pub server_addr: String,
}

impl Config {
    /// Create a new Config instance.
    pub(crate) fn new(config_file: Option<&PathBuf>) -> Result<Self> {
        // Setup defaults
        let mut figment = Figment::new()
            .merge(Serialized::default("log_format", "json"))
            .merge(Serialized::default("server_addr", "localhost:9000"));

        // Merge the config file provided (it must exist at the exact path)
        if let Some(config_file) = config_file {
            figment = figment.merge(Yaml::file_exact(config_file));
        }

        // Merge the environment variables, which take precedence
        let env = Env::prefixed("DCO2_").split("__");
        let cfg: Self = figment.merge(env_with_raw_keys(&env, RAW_ENV_KEYS)).extract()?;

        // Validate the resulting configuration
        cfg.github_app.validate().context("invalid configuration")?;

        Ok(cfg)
    }
}

/// Format to use in logs.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LogFormat {
    Json,
    Pretty,
}

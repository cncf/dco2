//! This module defines some types to represent the server configuration.

use std::path::PathBuf;

use anyhow::Result;
use figment::{
    providers::{Env, Format, Serialized, Yaml},
    Figment,
};
use serde::{Deserialize, Serialize};

use dco2::github::AppConfig;

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
        let mut figment = Figment::new()
            .merge(Serialized::default("log_format", "json"))
            .merge(Serialized::default("server_addr", "localhost:9000"));
        if let Some(config_file) = config_file {
            figment = figment.merge(Yaml::file(config_file));
        }
        figment.merge(Env::prefixed("DCO2_").split("__")).extract().map_err(Into::into)
    }
}

/// Format to use in logs.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LogFormat {
    Json,
    Pretty,
}

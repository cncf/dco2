use std::{env::set_var, sync::Arc};

use anyhow::Context;
use figment::{Figment, providers::Env};
use lambda_http::{Error, run, tracing};

use dco2::github::{AppConfig, GHClientOctorust};
use dco2_server::{env::env_with_raw_keys, handlers::setup_router};

/// Configuration keys whose environment variables values must not be parsed.
const RAW_ENV_KEYS: &[&str] = &["webhook_secret"];

#[tokio::main]
async fn main() -> Result<(), Error> {
    unsafe {
        // Do not include stage name in path
        set_var("AWS_LAMBDA_HTTP_IGNORE_STAGE_IN_PATH", "true");

        // Setup logging
        set_var("AWS_LAMBDA_LOG_FORMAT", "json");
    }
    tracing::init_default_subscriber();

    // Setup configuration
    let cfg: AppConfig = Figment::new()
        .merge(env_with_raw_keys(&Env::prefixed("DCO2_"), RAW_ENV_KEYS))
        .extract()
        .context("error setting up configuration")?;
    cfg.validate().context("invalid configuration")?;

    // Setup GitHub client
    let gh_client = Arc::new(GHClientOctorust::new(&cfg).context("error setting up github client")?);

    // Start lambda runtime
    let router = setup_router(gh_client, &cfg.webhook_secret);
    run(router).await
}

use std::{env::set_var, sync::Arc};

use anyhow::Context;
use figment::{providers::Env, Figment};
use lambda_http::{run, tracing, Error};

use dco2::github::{AppConfig, GHClientOctorust};
use dco2_server::handlers::setup_router;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Do not include stage name in path
    set_var("AWS_LAMBDA_HTTP_IGNORE_STAGE_IN_PATH", "true");

    // Setup logging
    set_var("AWS_LAMBDA_LOG_FORMAT", "json");
    tracing::init_default_subscriber();

    // Setup GitHub client
    let cfg: AppConfig = Figment::new()
        .merge(Env::prefixed("DCO2_"))
        .extract()
        .context("error setting up configuration")?;
    let gh_client = Arc::new(GHClientOctorust::new(&cfg).context("error setting up github client")?);

    // Start lambda runtime
    let router = setup_router(gh_client, &cfg.webhook_secret);
    run(router).await
}

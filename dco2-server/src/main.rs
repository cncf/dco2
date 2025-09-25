#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::doc_markdown, clippy::similar_names)]

use std::{path::PathBuf, sync::Arc};

use anyhow::{Context, Result};
use clap::Parser;
use config::{Config, LogFormat};
use tokio::{net::TcpListener, signal};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use dco2::github::GHClientOctorust;
use dco2_server::handlers::setup_router;

mod config;

#[derive(Debug, Parser)]
#[clap(author, version, about)]
struct Args {
    /// Config file path
    #[clap(short, long)]
    config_file: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Setup configuration
    let args = Args::parse();
    let cfg = Config::new(args.config_file.as_ref()).context("error setting up configuration")?;

    // Setup logging
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("dco2_server=debug,dco2=debug"));
    let ts = tracing_subscriber::fmt().with_env_filter(env_filter);
    match cfg.log_format {
        LogFormat::Json => ts.json().init(),
        LogFormat::Pretty => ts.init(),
    }

    // Setup GitHub client
    let gh_client = GHClientOctorust::new(&cfg.github_app).context("error setting up github client")?;
    let gh_client = Arc::new(gh_client);

    // Setup and launch HTTP server
    let router = setup_router(gh_client, &cfg.github_app.webhook_secret);
    let listener = TcpListener::bind(&cfg.server_addr).await?;
    info!("server started");
    info!(%cfg.server_addr, "listening");
    if let Err(err) = axum::serve(listener, router).with_graceful_shutdown(shutdown_signal()).await {
        error!(?err, "server error");
        return Err(err.into());
    }
    info!("server stopped");

    Ok(())
}

/// Return a future that will complete when the program is asked to stop via a
/// ctrl+c or terminate signal.
async fn shutdown_signal() {
    // Setup signal handlers
    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to install ctrl+c signal handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install terminate signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    // Wait for any of the signals
    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}

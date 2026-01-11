//! Zigroot CLI - Modern embedded Linux rootfs builder
//!
//! Entry point for the zigroot command-line application.

use anyhow::Result;
use clap::Parser;

use zigroot::cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing subscriber
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::WARN.into()),
        )
        .init();

    let cli = Cli::parse();
    cli.run().await
}

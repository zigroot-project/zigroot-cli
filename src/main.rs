//! Zigroot CLI - Modern embedded Linux rootfs builder
//!
//! Entry point for the zigroot command-line application.

use anyhow::Result;
use clap::Parser;

use zigroot::cli::output::{display_error, OutputConfig};
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

    // Apply output configuration globally
    let output_config = OutputConfig::new(cli.quiet, cli.json, cli.verbose);
    output_config.apply_global();

    // Run the command and handle errors
    match cli.run().await {
        Ok(()) => Ok(()),
        Err(e) => {
            display_error(&e);
            std::process::exit(1);
        }
    }
}

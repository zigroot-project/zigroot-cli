//! Command-line interface module
//!
//! This module handles argument parsing and output formatting.
//! It contains no business logic - that belongs in the [`crate::core`] module.

pub mod commands;
pub mod output;

use anyhow::Result;
use clap::Parser;

use commands::Commands;

/// Zigroot - Modern embedded Linux rootfs builder
///
/// Build embedded Linux root filesystems using Zig's cross-compilation.
#[derive(Parser, Debug)]
#[command(name = "zigroot")]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Enable verbose output (-v for info, -vv for debug)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Suppress all output except errors
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Output in JSON format for scripting
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

impl Cli {
    /// Execute the CLI command
    pub async fn run(self) -> Result<()> {
        if let Some(cmd) = self.command {
            cmd.run().await
        } else {
            // No subcommand provided, show help
            use clap::CommandFactory;
            let mut cmd = Self::command();
            cmd.print_help()?;
            Ok(())
        }
    }
}

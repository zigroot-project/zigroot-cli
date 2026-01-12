//! CLI command implementations
//!
//! Each command is implemented in its own submodule.

pub mod add;
pub mod init;
pub mod remove;

use anyhow::Result;
use clap::Subcommand;

/// Available CLI commands
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize a new zigroot project
    Init {
        /// Target board name
        #[arg(short, long)]
        board: Option<String>,

        /// Force initialization in non-empty directory
        #[arg(short, long)]
        force: bool,
    },

    /// Add a package to the project
    Add {
        /// Package name (optionally with @version)
        package: String,

        /// Add from git repository
        #[arg(long)]
        git: Option<String>,

        /// Use custom registry
        #[arg(long)]
        registry: Option<String>,
    },

    /// Remove a package from the project
    Remove {
        /// Package name to remove
        package: String,
    },

    /// Update packages to newer versions
    Update {
        /// Specific package to update (updates all if not specified)
        package: Option<String>,

        /// Check for zigroot updates
        #[arg(long)]
        self_update: bool,
    },

    /// Download package sources
    Fetch {
        /// Number of parallel downloads
        #[arg(short, long, default_value = "4")]
        parallel: usize,

        /// Force re-download even if files exist
        #[arg(short, long)]
        force: bool,
    },

    /// Build the rootfs
    Build {
        /// Build only specified package
        #[arg(short, long)]
        package: Option<String>,

        /// Number of parallel jobs
        #[arg(short, long)]
        jobs: Option<usize>,

        /// Fail if packages differ from lock file
        #[arg(long)]
        locked: bool,

        /// Enable binary compression
        #[arg(long)]
        compress: bool,

        /// Disable binary compression
        #[arg(long)]
        no_compress: bool,
    },

    /// Remove build artifacts
    Clean,

    /// Validate configuration without building
    Check,

    /// Search for packages and boards
    Search {
        /// Search query
        query: String,

        /// Search only packages
        #[arg(long)]
        packages: bool,

        /// Search only boards
        #[arg(long)]
        boards: bool,

        /// Force refresh of index
        #[arg(long)]
        refresh: bool,
    },

    /// Flash image to device
    Flash {
        /// Flash method to use
        method: Option<String>,

        /// Device path
        #[arg(short, long)]
        device: Option<String>,

        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,

        /// List available flash methods
        #[arg(short, long)]
        list: bool,
    },

    /// Package management subcommands
    Package {
        #[command(subcommand)]
        command: PackageCommands,
    },

    /// Board management subcommands
    Board {
        #[command(subcommand)]
        command: BoardCommands,
    },

    /// Display dependency tree
    Tree {
        /// Output in DOT graph format
        #[arg(long)]
        graph: bool,
    },

    /// Manage external artifacts
    External {
        #[command(subcommand)]
        command: ExternalCommands,
    },

    /// Check system dependencies
    Doctor,

    /// Generate standalone SDK
    Sdk {
        /// Output path for SDK tarball
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Display license information
    License {
        /// Export license report
        #[arg(long)]
        export: Option<String>,

        /// Generate SPDX SBOM
        #[arg(long)]
        sbom: bool,
    },

    /// Manage build cache
    Cache {
        #[command(subcommand)]
        command: CacheCommands,
    },

    /// Interactive configuration (TUI)
    Config,

    /// Verify package or board definition
    Verify {
        /// Path to package or board directory
        path: String,

        /// Also fetch and verify checksums
        #[arg(long)]
        fetch: bool,
    },

    /// Publish package or board to registry
    Publish {
        /// Path to package or board directory
        path: String,
    },

    /// Kernel management subcommands
    Kernel {
        #[command(subcommand)]
        command: KernelCommands,
    },
}

/// Package subcommands
#[derive(Subcommand, Debug)]
pub enum PackageCommands {
    /// List installed packages
    List,

    /// Show package information
    Info {
        /// Package name
        package: String,
    },

    /// Create a new package template
    New {
        /// Package name
        name: String,
    },

    /// Test build a package
    Test {
        /// Path to package directory
        path: String,
    },

    /// Bump package version
    Bump {
        /// Path to package directory
        path: String,

        /// New version
        version: String,
    },
}

/// Board subcommands
#[derive(Subcommand, Debug)]
pub enum BoardCommands {
    /// List available boards
    List,

    /// Set target board
    Set {
        /// Board name
        board: String,
    },

    /// Show board information
    Info {
        /// Board name
        board: String,
    },

    /// Create a new board template
    New {
        /// Board name
        name: String,
    },
}

/// External artifact subcommands
#[derive(Subcommand, Debug)]
pub enum ExternalCommands {
    /// List external artifacts
    List,

    /// Add external artifact
    Add {
        /// Artifact name
        name: String,

        /// Artifact type
        #[arg(long, value_name = "TYPE")]
        artifact_type: String,

        /// Remote URL
        #[arg(long)]
        url: Option<String>,

        /// Local path
        #[arg(long)]
        path: Option<String>,
    },
}

/// Cache subcommands
#[derive(Subcommand, Debug)]
pub enum CacheCommands {
    /// Show cache information
    Info,

    /// Clear cache
    Clean,

    /// Export cache to tarball
    Export {
        /// Output path
        output: String,
    },

    /// Import cache from tarball
    Import {
        /// Input path
        input: String,
    },
}

/// Kernel subcommands
#[derive(Subcommand, Debug)]
pub enum KernelCommands {
    /// Launch kernel menuconfig
    Menuconfig,
}

impl Commands {
    /// Execute the command
    pub async fn run(self) -> Result<()> {
        match self {
            Self::Init { board, force } => {
                let current_dir = std::env::current_dir()?;
                init::execute(&current_dir, board, force).await
            }
            Self::Add {
                package,
                git,
                registry,
            } => {
                let current_dir = std::env::current_dir()?;
                add::execute(&current_dir, &package, git, registry).await
            }
            Self::Remove { package } => {
                let current_dir = std::env::current_dir()?;
                remove::execute(&current_dir, &package).await
            }
            _ => {
                tracing::info!("Command not yet implemented");
                Ok(())
            }
        }
    }
}

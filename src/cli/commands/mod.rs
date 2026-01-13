//! CLI command implementations
//!
//! Each command is implemented in its own submodule.

pub mod add;
pub mod board;
pub mod build;
pub mod cache;
pub mod check;
pub mod clean;
pub mod config;
pub mod doctor;
pub mod external;
pub mod fetch;
pub mod flash;
pub mod init;
pub mod license;
pub mod package;
pub mod publish;
pub mod remove;
pub mod sdk;
pub mod search;
pub mod tree;
pub mod update;
pub mod verify;

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
        /// Show dependencies for specific package
        package: Option<String>,

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
    Config {
        /// Show only board selection
        #[arg(long)]
        board: bool,

        /// Show only package selection
        #[arg(long)]
        packages: bool,
    },

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

        /// New version number
        #[arg(value_name = "VERSION")]
        new_version: String,
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
            Self::Update { package, self_update: _ } => {
                let current_dir = std::env::current_dir()?;
                update::execute(&current_dir, package).await
            }
            Self::Fetch { parallel, force } => {
                let current_dir = std::env::current_dir()?;
                fetch::execute(&current_dir, parallel, force).await
            }
            Self::Build { package, jobs, locked, compress, no_compress } => {
                let current_dir = std::env::current_dir()?;
                let options = build::BuildOptions {
                    package,
                    jobs,
                    locked,
                    compress,
                    no_compress,
                };
                build::execute(&current_dir, options).await
            }
            Self::Clean => {
                let current_dir = std::env::current_dir()?;
                clean::execute(&current_dir).await
            }
            Self::Check => {
                let current_dir = std::env::current_dir()?;
                check::execute(&current_dir).await
            }
            Self::Search {
                query,
                packages,
                boards,
                refresh,
            } => {
                search::execute(&query, packages, boards, refresh).await
            }
            Self::Package { command } => {
                let current_dir = std::env::current_dir()?;
                match command {
                    PackageCommands::List => {
                        package::execute_list(&current_dir).await
                    }
                    PackageCommands::Info { package: pkg_name } => {
                        package::execute_info(&current_dir, &pkg_name).await
                    }
                    PackageCommands::New { name } => {
                        package::execute_new(&current_dir, &name).await
                    }
                    PackageCommands::Test { path } => {
                        package::execute_test(&current_dir, &path).await
                    }
                    PackageCommands::Bump { path, new_version } => {
                        package::execute_bump(&current_dir, &path, &new_version).await
                    }
                }
            }
            Self::Board { command } => {
                let current_dir = std::env::current_dir()?;
                match command {
                    BoardCommands::List => {
                        board::execute_list().await
                    }
                    BoardCommands::Set { board: board_name } => {
                        board::execute_set(&current_dir, &board_name).await
                    }
                    BoardCommands::Info { board: board_name } => {
                        board::execute_info(&board_name).await
                    }
                    BoardCommands::New { name } => {
                        board::execute_new(&current_dir, &name).await
                    }
                }
            }
            Self::Tree { package, graph } => {
                let current_dir = std::env::current_dir()?;
                tree::execute(&current_dir, package, graph).await
            }
            Self::Flash { method, device, yes, list } => {
                let current_dir = std::env::current_dir()?;
                flash::execute(&current_dir, method, device, yes, list).await
            }
            Self::External { command } => {
                let current_dir = std::env::current_dir()?;
                match command {
                    ExternalCommands::List => {
                        external::execute_list(&current_dir).await
                    }
                    ExternalCommands::Add { name, artifact_type, url, path } => {
                        external::execute_add(
                            &current_dir,
                            &name,
                            &artifact_type,
                            url.as_deref(),
                            path.as_deref(),
                        ).await
                    }
                }
            }
            Self::Doctor => {
                let current_dir = std::env::current_dir().ok();
                doctor::execute(current_dir.as_deref()).await
            }
            Self::Sdk { output } => {
                let current_dir = std::env::current_dir()?;
                sdk::execute(&current_dir, output).await
            }
            Self::License { export, sbom } => {
                let current_dir = std::env::current_dir()?;
                license::execute(&current_dir, export, sbom).await
            }
            Self::Cache { command } => {
                let current_dir = std::env::current_dir()?;
                match command {
                    CacheCommands::Info => {
                        cache::execute_info(&current_dir).await
                    }
                    CacheCommands::Clean => {
                        cache::execute_clean(&current_dir).await
                    }
                    CacheCommands::Export { output } => {
                        cache::execute_export(&current_dir, &output).await
                    }
                    CacheCommands::Import { input } => {
                        cache::execute_import(&current_dir, &input).await
                    }
                }
            }
            Self::Config { board, packages } => {
                let current_dir = std::env::current_dir()?;
                config::execute(&current_dir, board, packages).await
            }
            Self::Verify { path, fetch } => {
                let current_dir = std::env::current_dir()?;
                verify::execute(&current_dir, &path, fetch).await
            }
            Self::Publish { path } => {
                let current_dir = std::env::current_dir()?;
                publish::execute(&current_dir, &path).await
            }
            _ => {
                tracing::info!("Command not yet implemented");
                Ok(())
            }
        }
    }
}

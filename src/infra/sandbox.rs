//! Build isolation using Docker/Podman containers
//!
//! Provides container-based isolation for package builds to prevent
//! malicious packages from harming the host system.
//!
//! **Validates: Requirements 27.1-27.9**

use std::path::PathBuf;
use thiserror::Error;

/// Sandbox-related errors
#[derive(Error, Debug, Clone, PartialEq)]
pub enum SandboxError {
    /// Container runtime not available
    #[error("Container runtime not available. Install Docker or Podman to use --sandbox")]
    RuntimeNotAvailable,

    /// Container runtime not found
    #[error("Neither Docker nor Podman found in PATH")]
    RuntimeNotFound,

    /// Container execution failed
    #[error("Container execution failed: {message}")]
    ExecutionFailed { message: String },

    /// Invalid configuration
    #[error("Invalid sandbox configuration: {message}")]
    InvalidConfig { message: String },
}

/// Container runtime type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerRuntime {
    /// Docker container runtime
    Docker,
    /// Podman container runtime
    Podman,
}

impl ContainerRuntime {
    /// Get the command name for this runtime
    pub fn command(&self) -> &'static str {
        match self {
            ContainerRuntime::Docker => "docker",
            ContainerRuntime::Podman => "podman",
        }
    }
}

/// Mount configuration for container volumes
#[derive(Debug, Clone, PartialEq)]
pub struct MountConfig {
    /// Host path to mount
    pub host_path: PathBuf,
    /// Container path to mount to
    pub container_path: PathBuf,
    /// Whether the mount is read-only
    pub read_only: bool,
}

impl MountConfig {
    /// Create a new read-only mount
    pub fn read_only(host_path: PathBuf, container_path: PathBuf) -> Self {
        Self {
            host_path,
            container_path,
            read_only: true,
        }
    }

    /// Create a new read-write mount
    pub fn read_write(host_path: PathBuf, container_path: PathBuf) -> Self {
        Self {
            host_path,
            container_path,
            read_only: false,
        }
    }
}

/// Sandbox configuration for a build
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// Whether sandbox is enabled
    pub enabled: bool,
    /// Whether network access is allowed
    pub network_enabled: bool,
    /// Container image to use
    pub image: String,
    /// Mount configurations
    pub mounts: Vec<MountConfig>,
    /// Working directory inside container
    pub workdir: PathBuf,
    /// Environment variables to pass
    pub env: Vec<(String, String)>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            network_enabled: false,
            image: "alpine:latest".to_string(),
            mounts: Vec::new(),
            workdir: PathBuf::from("/build"),
            env: Vec::new(),
        }
    }
}

impl SandboxConfig {
    /// Create a new sandbox config with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable the sandbox
    pub fn enable(mut self) -> Self {
        self.enabled = true;
        self
    }

    /// Disable the sandbox
    pub fn disable(mut self) -> Self {
        self.enabled = false;
        self
    }

    /// Enable network access
    pub fn with_network(mut self) -> Self {
        self.network_enabled = true;
        self
    }

    /// Disable network access
    pub fn without_network(mut self) -> Self {
        self.network_enabled = false;
        self
    }

    /// Set the container image
    pub fn with_image(mut self, image: impl Into<String>) -> Self {
        self.image = image.into();
        self
    }

    /// Add a mount configuration
    pub fn with_mount(mut self, mount: MountConfig) -> Self {
        self.mounts.push(mount);
        self
    }

    /// Set the working directory
    pub fn with_workdir(mut self, workdir: PathBuf) -> Self {
        self.workdir = workdir;
        self
    }

    /// Add an environment variable
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }
}

/// Sandbox manager for running isolated builds
#[derive(Debug)]
pub struct Sandbox {
    /// Detected container runtime
    runtime: Option<ContainerRuntime>,
    /// Configuration
    config: SandboxConfig,
}

impl Sandbox {
    /// Create a new sandbox with the given configuration
    pub fn new(config: SandboxConfig) -> Self {
        Self {
            runtime: None,
            config,
        }
    }

    /// Detect available container runtime
    pub fn detect_runtime() -> Option<ContainerRuntime> {
        // Check for Docker first
        if Self::is_runtime_available(ContainerRuntime::Docker) {
            return Some(ContainerRuntime::Docker);
        }

        // Check for Podman
        if Self::is_runtime_available(ContainerRuntime::Podman) {
            return Some(ContainerRuntime::Podman);
        }

        None
    }

    /// Check if a specific runtime is available
    pub fn is_runtime_available(runtime: ContainerRuntime) -> bool {
        std::process::Command::new(runtime.command())
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Initialize the sandbox, detecting runtime
    pub fn init(&mut self) -> Result<(), SandboxError> {
        if !self.config.enabled {
            return Ok(());
        }

        self.runtime = Self::detect_runtime();
        if self.runtime.is_none() {
            return Err(SandboxError::RuntimeNotAvailable);
        }

        Ok(())
    }

    /// Get the detected runtime
    pub fn runtime(&self) -> Option<ContainerRuntime> {
        self.runtime
    }

    /// Get the configuration
    pub fn config(&self) -> &SandboxConfig {
        &self.config
    }

    /// Check if sandbox is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Build the container run command arguments
    pub fn build_run_args(&self, command: &[String]) -> Result<Vec<String>, SandboxError> {
        let _runtime = self.runtime.ok_or(SandboxError::RuntimeNotFound)?;

        let mut args = vec!["run".to_string(), "--rm".to_string()];

        // Network configuration
        if !self.config.network_enabled {
            args.push("--network=none".to_string());
        }

        // Mount configurations
        for mount in &self.config.mounts {
            let mount_opt = if mount.read_only {
                format!(
                    "-v={}:{}:ro",
                    mount.host_path.display(),
                    mount.container_path.display()
                )
            } else {
                format!(
                    "-v={}:{}",
                    mount.host_path.display(),
                    mount.container_path.display()
                )
            };
            args.push(mount_opt);
        }

        // Working directory
        args.push(format!("-w={}", self.config.workdir.display()));

        // Environment variables
        for (key, value) in &self.config.env {
            args.push(format!("-e={}={}", key, value));
        }

        // Image
        args.push(self.config.image.clone());

        // Command to run
        args.extend(command.iter().cloned());

        Ok(args)
    }

    /// Run a command in the sandbox
    pub fn run(&self, command: &[String]) -> Result<std::process::Output, SandboxError> {
        if !self.config.enabled {
            return Err(SandboxError::InvalidConfig {
                message: "Sandbox is not enabled".to_string(),
            });
        }

        let runtime = self.runtime.ok_or(SandboxError::RuntimeNotFound)?;
        let args = self.build_run_args(command)?;

        std::process::Command::new(runtime.command())
            .args(&args)
            .output()
            .map_err(|e| SandboxError::ExecutionFailed {
                message: e.to_string(),
            })
    }
}

/// Determine sandbox configuration from CLI flags and manifest settings
///
/// Priority: CLI flags > manifest settings > default (disabled)
pub fn resolve_sandbox_config(
    cli_sandbox: Option<bool>,
    cli_no_sandbox: bool,
    manifest_sandbox: Option<bool>,
    package_network: bool,
) -> SandboxConfig {
    let enabled = if cli_no_sandbox {
        // --no-sandbox always disables
        false
    } else if let Some(cli) = cli_sandbox {
        // CLI flag takes precedence
        cli
    } else {
        // Fall back to manifest setting, default to false
        manifest_sandbox.unwrap_or(false)
    };

    SandboxConfig {
        enabled,
        network_enabled: package_network,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_config_default() {
        let config = SandboxConfig::default();
        assert!(!config.enabled);
        assert!(!config.network_enabled);
        assert_eq!(config.image, "alpine:latest");
    }

    #[test]
    fn test_sandbox_config_builder() {
        let config = SandboxConfig::new()
            .enable()
            .with_network()
            .with_image("ubuntu:22.04")
            .with_workdir(PathBuf::from("/workspace"));

        assert!(config.enabled);
        assert!(config.network_enabled);
        assert_eq!(config.image, "ubuntu:22.04");
        assert_eq!(config.workdir, PathBuf::from("/workspace"));
    }

    #[test]
    fn test_mount_config() {
        let ro_mount = MountConfig::read_only(
            PathBuf::from("/host/src"),
            PathBuf::from("/container/src"),
        );
        assert!(ro_mount.read_only);

        let rw_mount = MountConfig::read_write(
            PathBuf::from("/host/build"),
            PathBuf::from("/container/build"),
        );
        assert!(!rw_mount.read_only);
    }

    #[test]
    fn test_container_runtime_command() {
        assert_eq!(ContainerRuntime::Docker.command(), "docker");
        assert_eq!(ContainerRuntime::Podman.command(), "podman");
    }

    #[test]
    fn test_resolve_sandbox_config_cli_no_sandbox_overrides_all() {
        // --no-sandbox should disable even if manifest enables
        let config = resolve_sandbox_config(Some(true), true, Some(true), false);
        assert!(!config.enabled);
    }

    #[test]
    fn test_resolve_sandbox_config_cli_sandbox_overrides_manifest() {
        // --sandbox should enable even if manifest disables
        let config = resolve_sandbox_config(Some(true), false, Some(false), false);
        assert!(config.enabled);
    }

    #[test]
    fn test_resolve_sandbox_config_manifest_fallback() {
        // Without CLI flags, use manifest setting
        let config = resolve_sandbox_config(None, false, Some(true), false);
        assert!(config.enabled);
    }

    #[test]
    fn test_resolve_sandbox_config_default_disabled() {
        // Without any settings, sandbox is disabled
        let config = resolve_sandbox_config(None, false, None, false);
        assert!(!config.enabled);
    }

    #[test]
    fn test_resolve_sandbox_config_network_from_package() {
        let config = resolve_sandbox_config(Some(true), false, None, true);
        assert!(config.enabled);
        assert!(config.network_enabled);
    }
}

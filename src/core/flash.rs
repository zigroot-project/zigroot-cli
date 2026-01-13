//! Flash command implementation
//!
//! Handles flashing images to devices using board-defined flash profiles.
//!
//! **Validates: Requirements 7.1-7.12**

use anyhow::{anyhow, bail, Context, Result};
use std::collections::HashMap;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use super::board::{BoardDefinition, FlashProfile};
use super::manifest::Manifest;

/// Flash options from CLI
#[derive(Debug, Clone)]
pub struct FlashOptions {
    /// Flash method to use
    pub method: Option<String>,
    /// Device path
    pub device: Option<String>,
    /// Skip confirmation prompt
    pub yes: bool,
    /// List available flash methods
    pub list: bool,
}

/// Result of a flash operation
#[derive(Debug)]
pub struct FlashResult {
    /// Flash method used
    pub method: String,
    /// Device path used
    pub device: Option<String>,
    /// Whether the flash was successful
    pub success: bool,
    /// Output message
    pub message: String,
}

/// Flash executor
pub struct FlashExecutor {
    /// Project root directory
    project_root: PathBuf,
    /// Manifest
    manifest: Manifest,
    /// Board definition (if available)
    board: Option<BoardDefinition>,
}

impl FlashExecutor {
    /// Create a new flash executor
    pub fn new(project_root: &Path, manifest: Manifest, board: Option<BoardDefinition>) -> Self {
        Self {
            project_root: project_root.to_path_buf(),
            manifest,
            board,
        }
    }

    /// Execute the flash command
    pub fn execute(&self, options: &FlashOptions) -> Result<FlashResult> {
        // If --list flag is set, list available methods
        if options.list {
            return self.list_flash_methods();
        }

        // Get available flash profiles
        let profiles = self.get_flash_profiles();

        // If no method specified, list available methods
        if options.method.is_none() {
            return self.list_flash_methods();
        }

        let method_name = options.method.as_ref().unwrap();

        // Find the requested flash profile
        let profile = profiles
            .iter()
            .find(|p| p.name == *method_name)
            .ok_or_else(|| {
                let available: Vec<_> = profiles.iter().map(|p| p.name.as_str()).collect();
                anyhow!(
                    "Unknown flash method '{}'. Available methods: {}",
                    method_name,
                    if available.is_empty() {
                        "none".to_string()
                    } else {
                        available.join(", ")
                    }
                )
            })?;

        // Check if image exists
        let image_path = self.get_image_path()?;
        if !image_path.exists() {
            bail!(
                "No rootfs image found at {}. Run 'zigroot build' first.",
                image_path.display()
            );
        }

        // Validate required tools
        self.validate_tools(profile)?;

        // Check required external artifacts
        self.check_required_artifacts(profile)?;

        // Require confirmation unless --yes is specified
        if !options.yes {
            self.require_confirmation(profile, options.device.as_deref())?;
        }

        // Execute the flash
        self.execute_flash(profile, options)
    }

    /// List available flash methods
    fn list_flash_methods(&self) -> Result<FlashResult> {
        let profiles = self.get_flash_profiles();

        if profiles.is_empty() {
            let message = if self.board.is_some() {
                "No flash methods defined for this board.\n\
                 Manual flashing instructions:\n\
                 1. Build your image with 'zigroot build'\n\
                 2. Copy the image from output/ to your device\n\
                 3. Use your device's native flashing tool"
                    .to_string()
            } else {
                "No board configured. Set a board with 'zigroot board set <board_name>' first."
                    .to_string()
            };

            return Ok(FlashResult {
                method: "list".to_string(),
                device: None,
                success: true,
                message,
            });
        }

        let mut message = String::from("Available flash methods:\n\n");
        for profile in &profiles {
            message.push_str(&format!("  {} - {}\n", profile.name, profile.description));
            if let Some(tool) = &profile.tool {
                message.push_str(&format!("    Tool: {}\n", tool));
            }
            if let Some(script) = &profile.script {
                message.push_str(&format!("    Script: {}\n", script));
            }
            if !profile.requires.is_empty() {
                message.push_str(&format!("    Requires: {}\n", profile.requires.join(", ")));
            }
            message.push('\n');
        }

        Ok(FlashResult {
            method: "list".to_string(),
            device: None,
            success: true,
            message,
        })
    }

    /// Get flash profiles from board definition
    fn get_flash_profiles(&self) -> Vec<FlashProfile> {
        self.board
            .as_ref()
            .map(|b| b.flash.clone())
            .unwrap_or_default()
    }

    /// Get the path to the rootfs image
    fn get_image_path(&self) -> Result<PathBuf> {
        let output_dir = self.project_root.join("output");
        let image_format = &self.manifest.build.image_format;

        let image_name = match image_format.as_str() {
            "ext4" => "rootfs.img",
            "squashfs" => "rootfs.squashfs",
            "initramfs" => "rootfs.cpio",
            _ => "rootfs.img",
        };

        Ok(output_dir.join(image_name))
    }

    /// Validate that required tools are installed
    fn validate_tools(&self, profile: &FlashProfile) -> Result<()> {
        if let Some(tool) = &profile.tool {
            // Check if the tool is available in PATH
            let status = Command::new("which").arg(tool).output();

            match status {
                Ok(output) if output.status.success() => Ok(()),
                _ => bail!(
                    "Required tool '{}' is not installed or not in PATH.\n\
                     Please install it before flashing.",
                    tool
                ),
            }
        } else {
            Ok(())
        }
    }

    /// Check that required external artifacts are available
    fn check_required_artifacts(&self, profile: &FlashProfile) -> Result<()> {
        for artifact_name in &profile.requires {
            let artifact = self.manifest.external.get(artifact_name);

            match artifact {
                Some(art) => {
                    // Check if artifact is available
                    if let Some(path) = &art.path {
                        let artifact_path = self.project_root.join(path);
                        if !artifact_path.exists() {
                            bail!(
                                "Required artifact '{}' not found at {}.\n\
                                 Run 'zigroot fetch' to download external artifacts.",
                                artifact_name,
                                artifact_path.display()
                            );
                        }
                    } else if art.url.is_some() {
                        // URL-based artifact - check in external/ directory
                        let external_dir = self.project_root.join("external");
                        let artifact_path = external_dir.join(artifact_name);
                        if !artifact_path.exists() {
                            bail!(
                                "Required artifact '{}' not downloaded.\n\
                                 Run 'zigroot fetch' to download external artifacts.",
                                artifact_name
                            );
                        }
                    }
                }
                None => {
                    bail!(
                        "Required artifact '{}' is not configured in zigroot.toml.\n\
                         Add it to the [external] section.",
                        artifact_name
                    );
                }
            }
        }

        Ok(())
    }

    /// Require user confirmation before flashing
    fn require_confirmation(&self, profile: &FlashProfile, device: Option<&str>) -> Result<()> {
        let device_str = device.unwrap_or("default device");

        eprintln!();
        eprintln!("⚠️  WARNING: This will flash to {}!", device_str);
        eprintln!("   Method: {} - {}", profile.name, profile.description);
        eprintln!();
        eprintln!("   This operation may cause data loss!");
        eprintln!();
        eprint!("   Are you sure you want to continue? [y/N] ");
        io::stderr().flush()?;

        // In non-interactive mode (no TTY), fail
        if !io::stdin().is_terminal() {
            bail!(
                "Cannot prompt for confirmation in non-interactive mode.\n\
                 Use --yes to skip confirmation."
            );
        }

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let input = input.trim().to_lowercase();
        if input != "y" && input != "yes" {
            bail!("Flash cancelled by user.");
        }

        Ok(())
    }

    /// Execute the flash operation
    fn execute_flash(&self, profile: &FlashProfile, options: &FlashOptions) -> Result<FlashResult> {
        let image_path = self.get_image_path()?;

        // Set up environment variables
        let mut env_vars: HashMap<String, String> = HashMap::new();
        env_vars.insert(
            "ZIGROOT_IMAGE".to_string(),
            image_path.to_string_lossy().to_string(),
        );
        env_vars.insert(
            "ZIGROOT_PROJECT".to_string(),
            self.project_root.to_string_lossy().to_string(),
        );

        if let Some(device) = &options.device {
            env_vars.insert("ZIGROOT_DEVICE".to_string(), device.clone());
        }

        // Add artifact paths to environment
        for artifact_name in &profile.requires {
            if let Some(artifact) = self.manifest.external.get(artifact_name) {
                let artifact_path = if let Some(path) = &artifact.path {
                    self.project_root.join(path)
                } else {
                    self.project_root.join("external").join(artifact_name)
                };

                let env_name = format!(
                    "ZIGROOT_ARTIFACT_{}",
                    artifact_name.to_uppercase().replace('-', "_")
                );
                env_vars.insert(env_name, artifact_path.to_string_lossy().to_string());
            }
        }

        // Execute based on profile type
        if let Some(script) = &profile.script {
            self.execute_script(script, &env_vars, options)
        } else if let Some(tool) = &profile.tool {
            self.execute_tool(tool, profile, &env_vars, options)
        } else {
            bail!(
                "Flash profile '{}' has neither script nor tool defined.",
                profile.name
            );
        }
    }

    /// Execute a flash script
    fn execute_script(
        &self,
        script: &str,
        env_vars: &HashMap<String, String>,
        options: &FlashOptions,
    ) -> Result<FlashResult> {
        let script_path = self.get_script_path(script)?;

        let mut cmd = Command::new(&script_path);
        cmd.current_dir(&self.project_root);

        for (key, value) in env_vars {
            cmd.env(key, value);
        }

        let output = cmd.output().with_context(|| {
            format!("Failed to execute flash script: {}", script_path.display())
        })?;

        let success = output.status.success();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let message = if success {
            format!("Flash completed successfully.\n{}", stdout)
        } else {
            format!("Flash failed.\n{}\n{}", stdout, stderr)
        };

        Ok(FlashResult {
            method: script.to_string(),
            device: options.device.clone(),
            success,
            message,
        })
    }

    /// Execute a flash tool
    fn execute_tool(
        &self,
        tool: &str,
        profile: &FlashProfile,
        env_vars: &HashMap<String, String>,
        options: &FlashOptions,
    ) -> Result<FlashResult> {
        let image_path = env_vars
            .get("ZIGROOT_IMAGE")
            .ok_or_else(|| anyhow!("ZIGROOT_IMAGE not set"))?;

        // Build command based on tool
        let mut cmd = Command::new(tool);
        cmd.current_dir(&self.project_root);

        for (key, value) in env_vars {
            cmd.env(key, value);
        }

        // Add tool-specific arguments
        match tool {
            "dd" => {
                cmd.arg(format!("if={}", image_path));
                if let Some(device) = &options.device {
                    cmd.arg(format!("of={}", device));
                } else {
                    bail!("Device path required for dd. Use --device <path>");
                }
                cmd.arg("bs=4M");
                cmd.arg("status=progress");
            }
            _ => {
                // Generic tool - pass image path as argument
                cmd.arg(image_path);
                if let Some(device) = &options.device {
                    cmd.arg(device);
                }
            }
        }

        let output = cmd
            .output()
            .with_context(|| format!("Failed to execute flash tool: {}", tool))?;

        let success = output.status.success();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let message = if success {
            format!("Flash completed successfully using {}.\n{}", tool, stdout)
        } else {
            format!("Flash failed using {}.\n{}\n{}", tool, stdout, stderr)
        };

        Ok(FlashResult {
            method: profile.name.clone(),
            device: options.device.clone(),
            success,
            message,
        })
    }

    /// Get the path to a flash script
    fn get_script_path(&self, script: &str) -> Result<PathBuf> {
        // Check in board directory first
        if let Some(board) = &self.board {
            let board_script = self
                .project_root
                .join("boards")
                .join(&board.board.name)
                .join(script);
            if board_script.exists() {
                return Ok(board_script);
            }
        }

        // Check in project root
        let project_script = self.project_root.join(script);
        if project_script.exists() {
            return Ok(project_script);
        }

        // Check in user/scripts
        let user_script = self.project_root.join("user").join("scripts").join(script);
        if user_script.exists() {
            return Ok(user_script);
        }

        bail!(
            "Flash script '{}' not found. Searched in:\n\
             - boards/<board_name>/{}\n\
             - {}\n\
             - user/scripts/{}",
            script,
            script,
            script,
            script
        );
    }
}

/// Load board definition from project
pub fn load_board_definition(project_root: &Path, board_name: &str) -> Result<BoardDefinition> {
    // Check local boards directory first
    let local_board_path = project_root
        .join("boards")
        .join(board_name)
        .join("board.toml");

    if local_board_path.exists() {
        let content = std::fs::read_to_string(&local_board_path).with_context(|| {
            format!(
                "Failed to read board definition: {}",
                local_board_path.display()
            )
        })?;
        return BoardDefinition::from_toml(&content).with_context(|| {
            format!(
                "Failed to parse board definition: {}",
                local_board_path.display()
            )
        });
    }

    bail!(
        "Board '{}' not found. Check that the board exists in boards/{}/board.toml",
        board_name,
        board_name
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flash_options_default() {
        let options = FlashOptions {
            method: None,
            device: None,
            yes: false,
            list: false,
        };

        assert!(options.method.is_none());
        assert!(options.device.is_none());
        assert!(!options.yes);
        assert!(!options.list);
    }

    #[test]
    fn test_flash_options_with_method() {
        let options = FlashOptions {
            method: Some("sd-card".to_string()),
            device: Some("/dev/sda".to_string()),
            yes: true,
            list: false,
        };

        assert_eq!(options.method, Some("sd-card".to_string()));
        assert_eq!(options.device, Some("/dev/sda".to_string()));
        assert!(options.yes);
        assert!(!options.list);
    }
}

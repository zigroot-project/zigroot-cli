//! Kernel build support
//!
//! Handles Linux kernel configuration, building, and module installation.
//!
//! # Overview
//!
//! The Linux kernel requires special handling compared to regular packages:
//! - Uses GCC toolchain instead of Zig
//! - Has its own configuration system (Kconfig)
//! - Builds kernel modules that need to be installed to /lib/modules/
//!
//! # Requirements
//!
//! - **26.9**: Supports defconfig for kernel configuration
//! - **26.10**: Supports config_fragments to customize configuration
//! - **26.14**: Builds kernel modules
//! - **26.15**: Installs modules to /lib/modules/<version>/

use std::path::PathBuf;

/// Kernel configuration
#[derive(Debug, Clone, Default)]
pub struct KernelConfig {
    /// Defconfig to use (e.g., "multi_v7_defconfig")
    pub defconfig: Option<String>,
    /// Config fragments to apply on top of defconfig
    pub config_fragments: Vec<String>,
}

impl KernelConfig {
    /// Create a new kernel configuration
    pub fn new(defconfig: Option<String>, config_fragments: Vec<String>) -> Self {
        Self {
            defconfig,
            config_fragments,
        }
    }

    /// Create a kernel configuration from a defconfig name
    pub fn from_defconfig(defconfig: &str) -> Self {
        Self {
            defconfig: Some(defconfig.to_string()),
            config_fragments: vec![],
        }
    }

    /// Check if this configuration has a defconfig
    pub fn has_defconfig(&self) -> bool {
        self.defconfig.is_some()
    }

    /// Check if this configuration has config fragments
    pub fn has_fragments(&self) -> bool {
        !self.config_fragments.is_empty()
    }
}

/// Kernel package representation
#[derive(Debug, Clone)]
pub struct KernelPackage {
    /// Package name
    name: String,
    /// Kernel version
    version: String,
    /// Kernel configuration
    config: KernelConfig,
}

impl KernelPackage {
    /// Create a new kernel package
    pub fn new(name: String, version: String, config: KernelConfig) -> Self {
        Self {
            name,
            version,
            config,
        }
    }

    /// Get the package name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the kernel version
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Get the defconfig name
    pub fn defconfig(&self) -> Option<&str> {
        self.config.defconfig.as_deref()
    }

    /// Get the config fragments
    pub fn config_fragments(&self) -> &[String] {
        &self.config.config_fragments
    }

    /// Get the kernel configuration
    pub fn config(&self) -> &KernelConfig {
        &self.config
    }

    /// Get the modules installation path for a kernel version
    pub fn modules_install_path(version: &str) -> String {
        format!("/lib/modules/{version}/")
    }

    /// Get the modules installation path for this kernel
    pub fn get_modules_install_path(&self) -> String {
        Self::modules_install_path(&self.version)
    }
}

/// Kernel build command generator
pub struct KernelBuildCommands;

impl KernelBuildCommands {
    /// Generate build commands for a kernel configuration
    ///
    /// # Arguments
    ///
    /// * `config` - The kernel configuration
    /// * `cross_compile` - The cross-compile prefix (e.g., "arm-linux-gnueabihf")
    ///
    /// # Returns
    ///
    /// A vector of shell commands to execute
    pub fn generate(config: &KernelConfig, cross_compile: &str) -> Vec<String> {
        let mut commands = Vec::new();
        let cross_prefix = format!("{cross_compile}-");

        // Step 1: Generate initial config from defconfig
        if let Some(defconfig) = &config.defconfig {
            commands.push(format!(
                "make ARCH=arm CROSS_COMPILE={cross_prefix} {defconfig}"
            ));
        }

        // Step 2: Apply config fragments if any
        if !config.config_fragments.is_empty() {
            let fragments = config.config_fragments.join(" ");
            commands.push(format!(
                "scripts/kconfig/merge_config.sh -m .config {fragments}"
            ));
            // Regenerate config after merging
            commands.push(format!(
                "make ARCH=arm CROSS_COMPILE={cross_prefix} olddefconfig"
            ));
        }

        // Step 3: Build kernel image
        commands.push(format!(
            "make ARCH=arm CROSS_COMPILE={cross_prefix} -j$(nproc) zImage"
        ));

        // Step 4: Build device tree blobs
        commands.push(format!(
            "make ARCH=arm CROSS_COMPILE={cross_prefix} -j$(nproc) dtbs"
        ));

        // Step 5: Build kernel modules
        commands.push(format!(
            "make ARCH=arm CROSS_COMPILE={cross_prefix} -j$(nproc) modules"
        ));

        // Step 6: Install kernel modules
        commands.push(format!(
            "make ARCH=arm CROSS_COMPILE={cross_prefix} INSTALL_MOD_PATH=$DESTDIR modules_install"
        ));

        commands
    }

    /// Generate commands for a specific architecture
    pub fn generate_for_arch(
        config: &KernelConfig,
        arch: &str,
        cross_compile: &str,
    ) -> Vec<String> {
        let mut commands = Vec::new();
        let cross_prefix = format!("{cross_compile}-");

        // Determine kernel image target based on architecture
        let image_target = match arch {
            "arm" => "zImage",
            "arm64" | "aarch64" => "Image",
            "x86_64" => "bzImage",
            "riscv64" => "Image",
            _ => "vmlinux",
        };

        // Step 1: Generate initial config from defconfig
        if let Some(defconfig) = &config.defconfig {
            commands.push(format!(
                "make ARCH={arch} CROSS_COMPILE={cross_prefix} {defconfig}"
            ));
        }

        // Step 2: Apply config fragments if any
        if !config.config_fragments.is_empty() {
            let fragments = config.config_fragments.join(" ");
            commands.push(format!(
                "scripts/kconfig/merge_config.sh -m .config {fragments}"
            ));
            commands.push(format!(
                "make ARCH={arch} CROSS_COMPILE={cross_prefix} olddefconfig"
            ));
        }

        // Step 3: Build kernel image
        commands.push(format!(
            "make ARCH={arch} CROSS_COMPILE={cross_prefix} -j$(nproc) {image_target}"
        ));

        // Step 4: Build device tree blobs (not for x86)
        if arch != "x86_64" {
            commands.push(format!(
                "make ARCH={arch} CROSS_COMPILE={cross_prefix} -j$(nproc) dtbs"
            ));
        }

        // Step 5: Build kernel modules
        commands.push(format!(
            "make ARCH={arch} CROSS_COMPILE={cross_prefix} -j$(nproc) modules"
        ));

        // Step 6: Install kernel modules
        commands.push(format!(
            "make ARCH={arch} CROSS_COMPILE={cross_prefix} INSTALL_MOD_PATH=$DESTDIR modules_install"
        ));

        commands
    }
}

/// Kernel build environment
#[derive(Debug, Clone)]
pub struct KernelBuildEnv {
    /// Source directory
    pub srcdir: PathBuf,
    /// Destination directory for installed files
    pub destdir: PathBuf,
    /// Architecture (e.g., "arm", "arm64")
    pub arch: String,
    /// Cross-compile prefix (e.g., "arm-linux-gnueabihf-")
    pub cross_compile: String,
    /// Number of parallel jobs
    pub jobs: usize,
    /// Path to kernel config directory (for saving menuconfig changes)
    pub config_dir: PathBuf,
}

impl KernelBuildEnv {
    /// Create a new kernel build environment
    pub fn new(
        srcdir: PathBuf,
        destdir: PathBuf,
        arch: String,
        cross_compile: String,
        config_dir: PathBuf,
    ) -> Self {
        Self {
            srcdir,
            destdir,
            arch,
            cross_compile,
            jobs: num_cpus::get(),
            config_dir,
        }
    }

    /// Get environment variables for kernel build
    pub fn to_env_vars(&self) -> std::collections::HashMap<String, String> {
        let mut env = std::collections::HashMap::new();
        env.insert("ARCH".to_string(), self.arch.clone());
        env.insert("CROSS_COMPILE".to_string(), self.cross_compile.clone());
        env.insert("SRCDIR".to_string(), self.srcdir.display().to_string());
        env.insert("DESTDIR".to_string(), self.destdir.display().to_string());
        env.insert(
            "INSTALL_MOD_PATH".to_string(),
            self.destdir.display().to_string(),
        );
        env.insert("JOBS".to_string(), self.jobs.to_string());
        env
    }

    /// Get the path where kernel config should be saved
    pub fn config_save_path(&self) -> PathBuf {
        self.config_dir.join(".config")
    }
}

/// Map target triple to kernel architecture
pub fn target_to_kernel_arch(target: &str) -> &'static str {
    if target.starts_with("arm-") || target.starts_with("armv7") {
        "arm"
    } else if target.starts_with("aarch64") || target.starts_with("arm64") {
        "arm64"
    } else if target.starts_with("x86_64") {
        "x86_64"
    } else if target.starts_with("riscv64") {
        "riscv"
    } else if target.starts_with("riscv32") {
        "riscv"
    } else {
        "unknown"
    }
}

/// Map target triple to cross-compile prefix
pub fn target_to_cross_compile(target: &str) -> String {
    match target {
        "arm-linux-gnueabihf" => "arm-linux-gnueabihf".to_string(),
        "arm-linux-musleabihf" => "arm-linux-musleabihf".to_string(),
        "aarch64-linux-gnu" => "aarch64-linux-gnu".to_string(),
        "aarch64-linux-musl" => "aarch64-linux-musl".to_string(),
        "x86_64-linux-gnu" => "x86_64-linux-gnu".to_string(),
        "x86_64-linux-musl" => "x86_64-linux-musl".to_string(),
        "riscv64-linux-gnu" => "riscv64-linux-gnu".to_string(),
        _ => target.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kernel_config_from_defconfig() {
        let config = KernelConfig::from_defconfig("multi_v7_defconfig");
        assert_eq!(config.defconfig, Some("multi_v7_defconfig".to_string()));
        assert!(config.config_fragments.is_empty());
    }

    #[test]
    fn test_kernel_config_with_fragments() {
        let config = KernelConfig::new(
            Some("sunxi_defconfig".to_string()),
            vec!["debug.config".to_string(), "custom.config".to_string()],
        );
        assert_eq!(config.defconfig, Some("sunxi_defconfig".to_string()));
        assert_eq!(config.config_fragments.len(), 2);
    }

    #[test]
    fn test_kernel_package_modules_path() {
        let path = KernelPackage::modules_install_path("6.6.0");
        assert_eq!(path, "/lib/modules/6.6.0/");
    }

    #[test]
    fn test_kernel_build_commands_with_defconfig() {
        let config = KernelConfig::from_defconfig("multi_v7_defconfig");
        let commands = KernelBuildCommands::generate(&config, "arm-linux-gnueabihf");

        assert!(!commands.is_empty());
        assert!(commands[0].contains("multi_v7_defconfig"));
        assert!(commands.iter().any(|c| c.contains("modules")));
        assert!(commands.iter().any(|c| c.contains("modules_install")));
    }

    #[test]
    fn test_kernel_build_commands_with_fragments() {
        let config = KernelConfig::new(
            Some("multi_v7_defconfig".to_string()),
            vec!["debug.config".to_string()],
        );
        let commands = KernelBuildCommands::generate(&config, "arm-linux-gnueabihf");

        assert!(commands.iter().any(|c| c.contains("merge_config")));
        assert!(commands.iter().any(|c| c.contains("debug.config")));
    }

    #[test]
    fn test_target_to_kernel_arch() {
        assert_eq!(target_to_kernel_arch("arm-linux-gnueabihf"), "arm");
        assert_eq!(target_to_kernel_arch("aarch64-linux-gnu"), "arm64");
        assert_eq!(target_to_kernel_arch("x86_64-linux-gnu"), "x86_64");
        assert_eq!(target_to_kernel_arch("riscv64-linux-gnu"), "riscv");
    }

    #[test]
    fn test_target_to_cross_compile() {
        assert_eq!(
            target_to_cross_compile("arm-linux-gnueabihf"),
            "arm-linux-gnueabihf"
        );
        assert_eq!(
            target_to_cross_compile("aarch64-linux-gnu"),
            "aarch64-linux-gnu"
        );
    }
}

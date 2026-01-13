//! Manifest (zigroot.toml) parsing and validation
//!
//! The manifest is the main configuration file for a zigroot project.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The main project manifest (zigroot.toml)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Manifest {
    /// Project configuration
    pub project: ProjectConfig,

    /// Board configuration
    #[serde(default)]
    pub board: BoardConfig,

    /// Build configuration
    #[serde(default)]
    pub build: BuildConfig,

    /// Package references
    #[serde(default)]
    pub packages: HashMap<String, PackageRef>,

    /// External artifacts
    #[serde(default)]
    pub external: HashMap<String, ExternalArtifact>,
}

/// Project-level configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectConfig {
    /// Project name
    pub name: String,

    /// Project version
    #[serde(default = "default_version")]
    pub version: String,

    /// Project description
    #[serde(default)]
    pub description: Option<String>,
}

fn default_version() -> String {
    "0.1.0".to_string()
}

/// Board configuration in the manifest
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct BoardConfig {
    /// Board name
    pub name: Option<String>,

    /// Board options overrides
    #[serde(default)]
    pub options: HashMap<String, toml::Value>,
}

/// Build configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BuildConfig {
    /// Enable binary compression
    #[serde(default)]
    pub compress: bool,

    /// Image format (ext4, squashfs, initramfs)
    #[serde(default = "default_image_format")]
    pub image_format: String,

    /// Root filesystem size
    #[serde(default = "default_rootfs_size")]
    pub rootfs_size: String,

    /// Hostname for the target system
    #[serde(default = "default_hostname")]
    pub hostname: String,

    /// Number of parallel jobs
    #[serde(default)]
    pub jobs: Option<usize>,

    /// Enable container isolation for builds
    /// **Validates: Requirement 27.3**
    #[serde(default)]
    pub sandbox: Option<bool>,
}

fn default_image_format() -> String {
    "ext4".to_string()
}

fn default_rootfs_size() -> String {
    "256M".to_string()
}

fn default_hostname() -> String {
    "zigroot".to_string()
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            compress: false,
            image_format: default_image_format(),
            rootfs_size: default_rootfs_size(),
            hostname: default_hostname(),
            jobs: None,
            sandbox: None,
        }
    }
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            name: "unnamed".to_string(),
            version: default_version(),
            description: None,
        }
    }
}

impl Default for Manifest {
    fn default() -> Self {
        Self {
            project: ProjectConfig::default(),
            board: BoardConfig::default(),
            build: BuildConfig::default(),
            packages: HashMap::new(),
            external: HashMap::new(),
        }
    }
}

/// Reference to a package in the manifest
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PackageRef {
    /// Version constraint
    #[serde(default)]
    pub version: Option<String>,

    /// Git repository URL
    #[serde(default)]
    pub git: Option<String>,

    /// Git ref (tag, branch, or rev)
    #[serde(default)]
    pub ref_: Option<String>,

    /// Custom registry URL
    #[serde(default)]
    pub registry: Option<String>,

    /// Package-specific options
    #[serde(default)]
    pub options: HashMap<String, toml::Value>,
}

/// External artifact configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExternalArtifact {
    /// Artifact type (bootloader, kernel, `partition_table`, dtb, firmware, other)
    #[serde(rename = "type")]
    pub artifact_type: String,

    /// Remote URL
    #[serde(default)]
    pub url: Option<String>,

    /// Local path
    #[serde(default)]
    pub path: Option<String>,

    /// SHA256 checksum (required for URL sources)
    #[serde(default)]
    pub sha256: Option<String>,

    /// Partition table format (gpt, mbr, rockchip)
    #[serde(default)]
    pub format: Option<String>,
}

impl Manifest {
    /// Load manifest from file path
    pub fn load(path: &std::path::Path) -> Result<Self, crate::error::ZigrootError> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            crate::error::ZigrootError::Io { source: e }
        })?;
        Self::from_toml(&content).map_err(|e| {
            crate::error::ZigrootError::ManifestParse { source: e }
        })
    }

    /// Load manifest from TOML string
    pub fn from_toml(content: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(content)
    }

    /// Serialize manifest to TOML string
    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // ============================================
    // Unit Tests
    // ============================================

    #[test]
    fn test_manifest_serializes_to_valid_toml() {
        let manifest = Manifest {
            project: ProjectConfig {
                name: "test-project".to_string(),
                version: "1.0.0".to_string(),
                description: Some("A test project".to_string()),
            },
            board: BoardConfig {
                name: Some("test-board".to_string()),
                options: HashMap::new(),
            },
            build: BuildConfig::default(),
            packages: HashMap::new(),
            external: HashMap::new(),
        };

        let toml_str = manifest.to_toml().expect("Failed to serialize");

        // Verify it's valid TOML by parsing it back
        let parsed: toml::Value = toml::from_str(&toml_str).expect("Output is not valid TOML");

        // Verify expected structure
        assert!(parsed.get("project").is_some());
        assert!(parsed.get("board").is_some());
        assert!(parsed.get("build").is_some());
    }

    #[test]
    fn test_manifest_deserializes_from_valid_toml() {
        let toml_content = r#"
[project]
name = "my-project"
version = "2.0.0"
description = "My embedded project"

[board]
name = "luckfox-pico"

[build]
compress = true
image_format = "squashfs"
rootfs_size = "128M"
hostname = "mydevice"

[packages.busybox]
version = "1.36.1"

[packages.dropbear]
git = "https://github.com/example/dropbear"
ref_ = "v2024.85"

[external.bootloader]
type = "bootloader"
url = "https://example.com/uboot.bin"
sha256 = "abc123"
"#;

        let manifest = Manifest::from_toml(toml_content).expect("Failed to parse valid TOML");

        assert_eq!(manifest.project.name, "my-project");
        assert_eq!(manifest.project.version, "2.0.0");
        assert_eq!(manifest.board.name, Some("luckfox-pico".to_string()));
        assert!(manifest.build.compress);
        assert_eq!(manifest.build.image_format, "squashfs");
        assert_eq!(manifest.packages.len(), 2);
        assert!(manifest.packages.contains_key("busybox"));
        assert!(manifest.packages.contains_key("dropbear"));
        assert_eq!(manifest.external.len(), 1);
    }

    #[test]
    fn test_manifest_roundtrip_basic() {
        let manifest = Manifest {
            project: ProjectConfig {
                name: "test-project".to_string(),
                version: "1.0.0".to_string(),
                description: Some("A test project".to_string()),
            },
            board: BoardConfig {
                name: Some("test-board".to_string()),
                options: HashMap::new(),
            },
            build: BuildConfig::default(),
            packages: HashMap::new(),
            external: HashMap::new(),
        };

        let toml_str = manifest.to_toml().expect("Failed to serialize");
        let parsed: Manifest = Manifest::from_toml(&toml_str).expect("Failed to parse");

        assert_eq!(manifest, parsed);
    }

    #[test]
    fn test_manifest_missing_required_project_name() {
        let toml_content = r#"
[project]
version = "1.0.0"

[board]
name = "test-board"

[build]
"#;

        let result = Manifest::from_toml(toml_content);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("name") || err.contains("missing"),
            "Error should mention missing 'name' field: {err}"
        );
    }

    #[test]
    fn test_manifest_missing_required_project_section() {
        let toml_content = r#"
[board]
name = "test-board"

[build]
compress = false
"#;

        let result = Manifest::from_toml(toml_content);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("project") || err.contains("missing"),
            "Error should mention missing 'project' section: {err}"
        );
    }

    #[test]
    fn test_manifest_with_packages_and_external() {
        let mut packages = HashMap::new();
        packages.insert(
            "busybox".to_string(),
            PackageRef {
                version: Some("1.36.1".to_string()),
                git: None,
                ref_: None,
                registry: None,
                options: HashMap::new(),
            },
        );

        let mut external = HashMap::new();
        external.insert(
            "bootloader".to_string(),
            ExternalArtifact {
                artifact_type: "bootloader".to_string(),
                url: Some("https://example.com/uboot.bin".to_string()),
                path: None,
                sha256: Some("abc123def456".to_string()),
                format: None,
            },
        );

        let manifest = Manifest {
            project: ProjectConfig {
                name: "complex-project".to_string(),
                version: "1.0.0".to_string(),
                description: None,
            },
            board: BoardConfig {
                name: Some("rpi4".to_string()),
                options: HashMap::new(),
            },
            build: BuildConfig {
                compress: true,
                image_format: "squashfs".to_string(),
                rootfs_size: "64M".to_string(),
                hostname: "mydevice".to_string(),
                jobs: Some(4),
                sandbox: None,
            },
            packages,
            external,
        };

        let toml_str = manifest.to_toml().expect("Failed to serialize");
        let parsed = Manifest::from_toml(&toml_str).expect("Failed to parse");

        assert_eq!(manifest, parsed);
    }

    #[test]
    fn test_manifest_default_values() {
        let toml_content = r#"
[project]
name = "minimal-project"
"#;

        let manifest = Manifest::from_toml(toml_content).expect("Failed to parse");

        // Check default values are applied
        assert_eq!(manifest.project.version, "0.1.0");
        assert_eq!(manifest.build.image_format, "ext4");
        assert_eq!(manifest.build.rootfs_size, "256M");
        assert_eq!(manifest.build.hostname, "zigroot");
        assert!(!manifest.build.compress);
    }

    // ============================================
    // Property-Based Tests
    // ============================================

    /// Strategy for generating valid project names
    fn project_name_strategy() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9_-]{0,30}[a-z0-9]?"
            .prop_filter("Name must not be empty", |s| !s.is_empty())
    }

    /// Strategy for generating valid semver versions
    fn version_strategy() -> impl Strategy<Value = String> {
        (1u32..100, 0u32..100, 0u32..100)
            .prop_map(|(major, minor, patch)| format!("{major}.{minor}.{patch}"))
    }

    /// Strategy for generating valid hostnames
    fn hostname_strategy() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9-]{0,20}[a-z0-9]?"
            .prop_filter("Hostname must not be empty", |s| !s.is_empty())
    }

    /// Strategy for generating valid image formats
    fn image_format_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("ext4".to_string()),
            Just("squashfs".to_string()),
            Just("initramfs".to_string()),
        ]
    }

    /// Strategy for generating valid rootfs sizes
    fn rootfs_size_strategy() -> impl Strategy<Value = String> {
        (1u32..1024, prop_oneof![Just("M"), Just("G")])
            .prop_map(|(size, unit)| format!("{size}{unit}"))
    }

    /// Strategy for generating optional descriptions
    fn description_strategy() -> impl Strategy<Value = Option<String>> {
        prop_oneof![
            Just(None),
            "[a-zA-Z0-9 ]{1,100}".prop_map(Some),
        ]
    }

    /// Strategy for generating a complete Manifest
    fn manifest_strategy() -> impl Strategy<Value = Manifest> {
        (
            project_name_strategy(),
            version_strategy(),
            description_strategy(),
            prop::option::of(project_name_strategy()), // board name
            prop::bool::ANY,                            // compress
            image_format_strategy(),
            rootfs_size_strategy(),
            hostname_strategy(),
            prop::option::of(1usize..32),              // jobs
        )
            .prop_map(
                |(name, version, description, board_name, compress, image_format, rootfs_size, hostname, jobs)| {
                    Manifest {
                        project: ProjectConfig {
                            name,
                            version,
                            description,
                        },
                        board: BoardConfig {
                            name: board_name,
                            options: HashMap::new(),
                        },
                        build: BuildConfig {
                            compress,
                            image_format,
                            rootfs_size,
                            hostname,
                            jobs,
                            sandbox: None,
                        },
                        packages: HashMap::new(),
                        external: HashMap::new(),
                    }
                },
            )
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 1: TOML Serialization Round-Trip (Manifest)
        /// For all valid Manifest configurations, serializing then deserializing
        /// SHALL produce an equivalent Manifest.
        /// **Validates: Requirements 16.1, 16.2, 16.3**
        #[test]
        fn prop_manifest_toml_roundtrip(manifest in manifest_strategy()) {
            // Serialize to TOML
            let toml_str = manifest.to_toml()
                .expect("Manifest should serialize to valid TOML");

            // Verify it's valid TOML
            let _: toml::Value = toml::from_str(&toml_str)
                .expect("Serialized output should be valid TOML");

            // Deserialize back
            let parsed = Manifest::from_toml(&toml_str)
                .expect("Should deserialize back to Manifest");

            // Verify equivalence
            prop_assert_eq!(manifest, parsed, "Round-trip should produce equivalent Manifest");
        }

        /// Property: Serialization produces valid TOML
        /// **Validates: Requirements 16.1**
        #[test]
        fn prop_manifest_serializes_to_valid_toml(manifest in manifest_strategy()) {
            let toml_str = manifest.to_toml()
                .expect("Manifest should serialize");

            // Should parse as valid TOML
            let result: Result<toml::Value, _> = toml::from_str(&toml_str);
            prop_assert!(result.is_ok(), "Output should be valid TOML: {:?}", result.err());
        }

        /// Property: Project name is preserved through serialization
        #[test]
        fn prop_project_name_preserved(name in project_name_strategy()) {
            let manifest = Manifest {
                project: ProjectConfig {
                    name: name.clone(),
                    version: "1.0.0".to_string(),
                    description: None,
                },
                board: BoardConfig::default(),
                build: BuildConfig::default(),
                packages: HashMap::new(),
                external: HashMap::new(),
            };

            let toml_str = manifest.to_toml().expect("Should serialize");
            let parsed = Manifest::from_toml(&toml_str).expect("Should parse");

            prop_assert_eq!(parsed.project.name, name);
        }
    }
}

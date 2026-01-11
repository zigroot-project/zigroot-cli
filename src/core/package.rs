//! Package definition handling
//!
//! Handles parsing of both local package.toml files and registry
//! metadata.toml + version.toml files.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complete package definition (merged from metadata + version for registry packages)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PackageDefinition {
    /// Package metadata
    pub package: PackageMetadata,

    /// Source configuration
    pub source: SourceConfig,

    /// Build configuration
    #[serde(default)]
    pub build: PackageBuildConfig,

    /// Package options
    #[serde(default)]
    pub options: HashMap<String, OptionDefinition>,

    /// Installation configuration
    #[serde(default)]
    pub install: InstallConfig,
}

/// Package metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PackageMetadata {
    /// Package name
    pub name: String,

    /// Package version
    pub version: String,

    /// Package description
    pub description: String,

    /// License identifier
    #[serde(default)]
    pub license: Option<String>,

    /// Homepage URL
    #[serde(default)]
    pub homepage: Option<String>,

    /// Search keywords
    #[serde(default)]
    pub keywords: Vec<String>,

    /// Build dependencies (packages needed at build time)
    #[serde(default)]
    pub depends: Vec<String>,

    /// Runtime dependencies (packages needed in final rootfs)
    #[serde(default)]
    pub requires: Vec<String>,

    /// Supported architectures (empty = all)
    #[serde(default)]
    pub arch: Vec<String>,

    /// Virtual packages this provides
    #[serde(default)]
    pub provides: Vec<String>,

    /// Conflicting packages
    #[serde(default)]
    pub conflicts: Vec<String>,

    /// Minimum zigroot version required
    #[serde(default)]
    pub zigroot_version: Option<String>,
}

/// Source configuration - exactly ONE source type must be specified
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum SourceConfig {
    /// URL source with checksum
    Url { url: String, sha256: String },

    /// Git source with ref
    Git {
        git: String,
        #[serde(flatten)]
        git_ref: GitRef,
    },

    /// Multiple source files
    Sources { sources: Vec<SourceFile> },
}

/// Git reference type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum GitRef {
    Tag(String),
    Branch(String),
    Rev(String),
}

/// Individual source file in multi-source packages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SourceFile {
    /// Source URL
    pub url: String,

    /// SHA256 checksum
    pub sha256: String,

    /// Destination filename
    #[serde(default)]
    pub filename: Option<String>,
}

/// Package build configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PackageBuildConfig {
    /// Build system type (autotools, cmake, meson, make, custom)
    #[serde(rename = "type")]
    #[serde(default)]
    pub build_type: Option<String>,

    /// Custom build steps
    #[serde(default)]
    pub steps: Vec<BuildStep>,

    /// Configure arguments
    #[serde(default)]
    pub configure_args: Vec<String>,

    /// Make arguments
    #[serde(default)]
    pub make_args: Vec<String>,

    /// `CMake` arguments
    #[serde(default)]
    pub cmake_args: Vec<String>,

    /// Patches to apply
    #[serde(default)]
    pub patches: Vec<String>,

    /// Whether this package requires network during build
    #[serde(default)]
    pub network: bool,

    /// Enable/disable compression for this package
    #[serde(default)]
    pub compress: Option<bool>,
}

/// A single build step
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BuildStep {
    /// Command to run
    pub run: String,

    /// Command arguments
    #[serde(default)]
    pub args: Vec<String>,
}

/// Option definition for configurable packages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OptionDefinition {
    /// Option type (bool, string, choice, number)
    #[serde(rename = "type")]
    pub option_type: String,

    /// Default value
    pub default: toml::Value,

    /// Description
    pub description: String,

    /// Valid choices (for choice type)
    #[serde(default)]
    pub choices: Vec<String>,

    /// Regex pattern (for string type)
    #[serde(default)]
    pub pattern: Option<String>,

    /// Allow empty string (for string type)
    #[serde(default = "default_true")]
    pub allow_empty: bool,

    /// Minimum value (for number type)
    #[serde(default)]
    pub min: Option<f64>,

    /// Maximum value (for number type)
    #[serde(default)]
    pub max: Option<f64>,
}

fn default_true() -> bool {
    true
}

/// Installation configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct InstallConfig {
    /// Custom install script
    #[serde(default)]
    pub script: Option<String>,

    /// Declarative file installation rules
    #[serde(default)]
    pub files: Vec<InstallRule>,
}

/// Declarative file installation rule
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InstallRule {
    /// Source path (relative to build directory)
    pub src: String,

    /// Destination path (relative to rootfs)
    pub dst: String,

    /// File mode (e.g., "755")
    #[serde(default)]
    pub mode: Option<String>,
}

impl PackageDefinition {
    /// Parse from TOML string (local package.toml format)
    pub fn from_toml(content: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(content)
    }

    /// Serialize to TOML string
    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // ============================================
    // Unit Tests - Local package.toml parsing
    // ============================================

    #[test]
    fn test_local_package_parses_correctly() {
        let toml_content = r#"
[package]
name = "busybox"
version = "1.36.1"
description = "Swiss army knife of embedded Linux"
license = "GPL-2.0"
homepage = "https://busybox.net"
keywords = ["shell", "coreutils", "init"]
depends = ["zlib"]
requires = []

[source]
url = "https://busybox.net/downloads/busybox-1.36.1.tar.bz2"
sha256 = "b8cc24c9574d809e7279c3be349795c5d5ceb6fdf19ca709f80cde50e47de314"

[build]
type = "make"
make_args = ["CROSS_COMPILE=${TARGET}-"]
"#;

        let pkg = PackageDefinition::from_toml(toml_content).expect("Failed to parse valid package");

        assert_eq!(pkg.package.name, "busybox");
        assert_eq!(pkg.package.version, "1.36.1");
        assert_eq!(pkg.package.description, "Swiss army knife of embedded Linux");
        assert_eq!(pkg.package.license, Some("GPL-2.0".to_string()));
        assert_eq!(pkg.package.depends, vec!["zlib"]);
        
        // Verify source is URL type
        match &pkg.source {
            SourceConfig::Url { url, sha256 } => {
                assert!(url.contains("busybox"));
                assert_eq!(sha256, "b8cc24c9574d809e7279c3be349795c5d5ceb6fdf19ca709f80cde50e47de314");
            }
            _ => panic!("Expected URL source type"),
        }
        
        assert_eq!(pkg.build.build_type, Some("make".to_string()));
    }

    #[test]
    fn test_package_with_git_source() {
        let toml_content = r#"
[package]
name = "custom-pkg"
version = "1.0.0"
description = "A custom package from git"

[source]
git = "https://github.com/example/repo"
tag = "v1.0.0"

[build]
type = "cmake"
"#;

        let pkg = PackageDefinition::from_toml(toml_content).expect("Failed to parse git package");

        match &pkg.source {
            SourceConfig::Git { git, git_ref } => {
                assert_eq!(git, "https://github.com/example/repo");
                match git_ref {
                    GitRef::Tag(tag) => assert_eq!(tag, "v1.0.0"),
                    _ => panic!("Expected tag ref"),
                }
            }
            _ => panic!("Expected Git source type"),
        }
    }

    #[test]
    fn test_package_with_git_branch() {
        let toml_content = r#"
[package]
name = "dev-pkg"
version = "0.1.0"
description = "Development package"

[source]
git = "https://github.com/example/repo"
branch = "main"
"#;

        let pkg = PackageDefinition::from_toml(toml_content).expect("Failed to parse");

        match &pkg.source {
            SourceConfig::Git { git_ref, .. } => {
                match git_ref {
                    GitRef::Branch(branch) => assert_eq!(branch, "main"),
                    _ => panic!("Expected branch ref"),
                }
            }
            _ => panic!("Expected Git source type"),
        }
    }

    #[test]
    fn test_package_with_git_rev() {
        let toml_content = r#"
[package]
name = "pinned-pkg"
version = "0.1.0"
description = "Pinned to specific commit"

[source]
git = "https://github.com/example/repo"
rev = "abc123def456"
"#;

        let pkg = PackageDefinition::from_toml(toml_content).expect("Failed to parse");

        match &pkg.source {
            SourceConfig::Git { git_ref, .. } => {
                match git_ref {
                    GitRef::Rev(rev) => assert_eq!(rev, "abc123def456"),
                    _ => panic!("Expected rev ref"),
                }
            }
            _ => panic!("Expected Git source type"),
        }
    }

    // ============================================
    // Round-trip tests
    // ============================================

    #[test]
    fn test_package_roundtrip_url_source() {
        let pkg = PackageDefinition {
            package: PackageMetadata {
                name: "test-pkg".to_string(),
                version: "1.0.0".to_string(),
                description: "Test package".to_string(),
                license: Some("MIT".to_string()),
                homepage: None,
                keywords: vec!["test".to_string()],
                depends: vec![],
                requires: vec![],
                arch: vec![],
                provides: vec![],
                conflicts: vec![],
                zigroot_version: None,
            },
            source: SourceConfig::Url {
                url: "https://example.com/test-1.0.0.tar.gz".to_string(),
                sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
            },
            build: PackageBuildConfig::default(),
            options: HashMap::new(),
            install: InstallConfig::default(),
        };

        let toml_str = pkg.to_toml().expect("Failed to serialize");
        let parsed = PackageDefinition::from_toml(&toml_str).expect("Failed to parse");

        assert_eq!(pkg, parsed);
    }

    // ============================================
    // Missing required fields tests
    // ============================================

    #[test]
    fn test_missing_package_name() {
        let toml_content = r#"
[package]
version = "1.0.0"
description = "Missing name"

[source]
url = "https://example.com/test.tar.gz"
sha256 = "abc123"
"#;

        let result = PackageDefinition::from_toml(toml_content);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("name") || err.contains("missing"),
            "Error should mention missing 'name': {err}"
        );
    }

    #[test]
    fn test_missing_package_version() {
        let toml_content = r#"
[package]
name = "test-pkg"
description = "Missing version"

[source]
url = "https://example.com/test.tar.gz"
sha256 = "abc123"
"#;

        let result = PackageDefinition::from_toml(toml_content);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("version") || err.contains("missing"),
            "Error should mention missing 'version': {err}"
        );
    }

    #[test]
    fn test_missing_package_description() {
        let toml_content = r#"
[package]
name = "test-pkg"
version = "1.0.0"

[source]
url = "https://example.com/test.tar.gz"
sha256 = "abc123"
"#;

        let result = PackageDefinition::from_toml(toml_content);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("description") || err.contains("missing"),
            "Error should mention missing 'description': {err}"
        );
    }

    #[test]
    fn test_missing_source_section() {
        let toml_content = r#"
[package]
name = "test-pkg"
version = "1.0.0"
description = "Missing source"
"#;

        let result = PackageDefinition::from_toml(toml_content);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("source") || err.contains("missing"),
            "Error should mention missing 'source': {err}"
        );
    }

    // ============================================
    // Source type validation tests
    // ============================================

    #[test]
    fn test_url_without_sha256_produces_error() {
        let toml_content = r#"
[package]
name = "test-pkg"
version = "1.0.0"
description = "URL without checksum"

[source]
url = "https://example.com/test.tar.gz"
"#;

        let result = PackageDefinition::from_toml(toml_content);
        // This should fail because URL requires sha256
        assert!(result.is_err(), "URL source without sha256 should fail");
    }

    #[test]
    fn test_git_without_ref_produces_error() {
        let toml_content = r#"
[package]
name = "test-pkg"
version = "1.0.0"
description = "Git without ref"

[source]
git = "https://github.com/example/repo"
"#;

        let result = PackageDefinition::from_toml(toml_content);
        // This should fail because git requires tag, branch, or rev
        assert!(result.is_err(), "Git source without ref should fail");
    }

    // ============================================
    // Property-Based Tests
    // ============================================

    /// Strategy for generating valid package names
    fn package_name_strategy() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9-]{0,30}[a-z0-9]?"
            .prop_filter("Name must not be empty", |s| !s.is_empty())
    }

    /// Strategy for generating valid semver versions
    fn version_strategy() -> impl Strategy<Value = String> {
        (1u32..100, 0u32..100, 0u32..100)
            .prop_map(|(major, minor, patch)| format!("{major}.{minor}.{patch}"))
    }

    /// Strategy for generating valid SHA256 hashes
    fn sha256_strategy() -> impl Strategy<Value = String> {
        "[0-9a-f]{64}"
    }

    /// Strategy for generating valid URLs
    fn url_strategy() -> impl Strategy<Value = String> {
        (
            "[a-z]{3,10}",
            "[a-z]{2,5}",
            "[a-z0-9-]{1,20}",
        )
            .prop_map(|(domain, tld, path)| {
                format!("https://{domain}.{tld}/{path}.tar.gz")
            })
    }

    /// Strategy for generating descriptions
    fn description_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9 ]{1,100}"
    }

    /// Strategy for generating URL source config
    fn url_source_strategy() -> impl Strategy<Value = SourceConfig> {
        (url_strategy(), sha256_strategy())
            .prop_map(|(url, sha256)| SourceConfig::Url { url, sha256 })
    }

    /// Strategy for generating a complete PackageDefinition with URL source
    fn package_definition_strategy() -> impl Strategy<Value = PackageDefinition> {
        (
            package_name_strategy(),
            version_strategy(),
            description_strategy(),
            url_source_strategy(),
        )
            .prop_map(|(name, version, description, source)| {
                PackageDefinition {
                    package: PackageMetadata {
                        name,
                        version,
                        description,
                        license: None,
                        homepage: None,
                        keywords: vec![],
                        depends: vec![],
                        requires: vec![],
                        arch: vec![],
                        provides: vec![],
                        conflicts: vec![],
                        zigroot_version: None,
                    },
                    source,
                    build: PackageBuildConfig::default(),
                    options: HashMap::new(),
                    install: InstallConfig::default(),
                }
            })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 1: TOML Serialization Round-Trip (PackageDefinition)
        /// For all valid PackageDefinition configurations, serializing then deserializing
        /// SHALL produce an equivalent PackageDefinition.
        /// **Validates: Requirements 17.1-17.5**
        #[test]
        fn prop_package_toml_roundtrip(pkg in package_definition_strategy()) {
            // Serialize to TOML
            let toml_str = pkg.to_toml()
                .expect("PackageDefinition should serialize to valid TOML");

            // Verify it's valid TOML
            let _: toml::Value = toml::from_str(&toml_str)
                .expect("Serialized output should be valid TOML");

            // Deserialize back
            let parsed = PackageDefinition::from_toml(&toml_str)
                .expect("Should deserialize back to PackageDefinition");

            // Verify equivalence
            prop_assert_eq!(pkg, parsed, "Round-trip should produce equivalent PackageDefinition");
        }

        /// Property: Package name is preserved through serialization
        #[test]
        fn prop_package_name_preserved(name in package_name_strategy()) {
            let pkg = PackageDefinition {
                package: PackageMetadata {
                    name: name.clone(),
                    version: "1.0.0".to_string(),
                    description: "Test".to_string(),
                    license: None,
                    homepage: None,
                    keywords: vec![],
                    depends: vec![],
                    requires: vec![],
                    arch: vec![],
                    provides: vec![],
                    conflicts: vec![],
                    zigroot_version: None,
                },
                source: SourceConfig::Url {
                    url: "https://example.com/test.tar.gz".to_string(),
                    sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
                },
                build: PackageBuildConfig::default(),
                options: HashMap::new(),
                install: InstallConfig::default(),
            };

            let toml_str = pkg.to_toml().expect("Should serialize");
            let parsed = PackageDefinition::from_toml(&toml_str).expect("Should parse");

            prop_assert_eq!(parsed.package.name, name);
        }

        /// Property 20: URL Checksum Requirement
        /// For all URL sources, sha256 must be present
        /// **Validates: Requirements 18.11**
        #[test]
        fn prop_url_source_has_sha256(url in url_strategy(), sha256 in sha256_strategy()) {
            let source = SourceConfig::Url { url: url.clone(), sha256: sha256.clone() };
            
            match source {
                SourceConfig::Url { sha256: hash, .. } => {
                    prop_assert!(!hash.is_empty(), "URL source must have non-empty sha256");
                    prop_assert_eq!(hash.len(), 64, "SHA256 must be 64 hex characters");
                }
                _ => prop_assert!(false, "Expected URL source"),
            }
        }
    }
}

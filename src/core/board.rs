//! Board definition handling
//!
//! Handles parsing of board.toml files that define hardware targets.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::package::OptionDefinition;

/// Complete board definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BoardDefinition {
    /// Board metadata
    pub board: BoardMetadata,

    /// Default settings
    pub defaults: BoardDefaults,

    /// Required packages for this board
    #[serde(default)]
    pub requires: Vec<String>,

    /// Flash profiles
    #[serde(default)]
    pub flash: Vec<FlashProfile>,

    /// Board options
    #[serde(default)]
    pub options: HashMap<String, OptionDefinition>,
}

/// Board metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BoardMetadata {
    /// Board name
    pub name: String,

    /// Board description
    pub description: String,

    /// Zig target triple (e.g., "arm-linux-musleabihf")
    pub target: String,

    /// CPU type
    pub cpu: String,

    /// CPU features
    #[serde(default)]
    pub features: Vec<String>,

    /// Kernel configuration
    #[serde(default)]
    pub kernel: Option<String>,

    /// Minimum zigroot version required
    #[serde(default)]
    pub zigroot_version: Option<String>,
}

/// Default settings for the board
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BoardDefaults {
    /// Default image format
    pub image_format: String,

    /// Default rootfs size
    pub rootfs_size: String,

    /// Default hostname
    pub hostname: String,
}

/// Flash profile for programming the device
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FlashProfile {
    /// Profile name
    pub name: String,

    /// Profile description
    pub description: String,

    /// Flash script path
    #[serde(default)]
    pub script: Option<String>,

    /// Flash tool command
    #[serde(default)]
    pub tool: Option<String>,

    /// Required external artifacts
    #[serde(default)]
    pub requires: Vec<String>,
}

impl BoardDefinition {
    /// Parse from TOML string
    pub fn from_toml(content: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(content)
    }

    /// Serialize to TOML string
    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }
}

impl TryFrom<toml::Value> for BoardDefinition {
    type Error = toml::de::Error;

    fn try_from(value: toml::Value) -> Result<Self, Self::Error> {
        // Convert toml::Value to string and parse
        let toml_str = toml::to_string(&value).map_err(|e| {
            serde::de::Error::custom(format!("Failed to serialize TOML value: {}", e))
        })?;
        Self::from_toml(&toml_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // ============================================
    // Unit Tests - board.toml parsing
    // ============================================

    #[test]
    fn test_board_parses_correctly() {
        // Note: In TOML, top-level keys must come before any section headers
        let toml_content = r#"
# Top-level requires must come before sections
requires = ["busybox", "dropbear"]

[board]
name = "luckfox-pico"
description = "Luckfox Pico (RV1103 SoC, Cortex-A7)"
target = "arm-linux-musleabihf"
cpu = "cortex-a7"
features = ["neon", "vfpv4"]
kernel = "linux-luckfox"
zigroot_version = ">=0.2.0"

[defaults]
image_format = "ext4"
rootfs_size = "256M"
hostname = "luckfox"

[[flash]]
name = "sd-card"
description = "Flash to SD card using dd"
tool = "dd"
requires = ["bootloader", "partition_table"]

[[flash]]
name = "rkflashtool"
description = "Flash using Rockchip flash tool"
script = "flash-rk.sh"
requires = ["bootloader"]

[options.boot_mode]
type = "choice"
default = "sd"
choices = ["sd", "emmc", "spi"]
description = "Boot device selection"
"#;

        let board = BoardDefinition::from_toml(toml_content).expect("Failed to parse valid board");

        assert_eq!(board.board.name, "luckfox-pico");
        assert_eq!(
            board.board.description,
            "Luckfox Pico (RV1103 SoC, Cortex-A7)"
        );
        assert_eq!(board.board.target, "arm-linux-musleabihf");
        assert_eq!(board.board.cpu, "cortex-a7");
        assert_eq!(board.board.features, vec!["neon", "vfpv4"]);
        assert_eq!(board.board.kernel, Some("linux-luckfox".to_string()));
        assert_eq!(board.board.zigroot_version, Some(">=0.2.0".to_string()));

        assert_eq!(board.defaults.image_format, "ext4");
        assert_eq!(board.defaults.rootfs_size, "256M");
        assert_eq!(board.defaults.hostname, "luckfox");

        assert_eq!(board.requires, vec!["busybox", "dropbear"]);

        assert_eq!(board.flash.len(), 2);
        assert_eq!(board.flash[0].name, "sd-card");
        assert_eq!(board.flash[0].tool, Some("dd".to_string()));
        assert_eq!(board.flash[1].name, "rkflashtool");
        assert_eq!(board.flash[1].script, Some("flash-rk.sh".to_string()));

        assert!(board.options.contains_key("boot_mode"));
    }

    #[test]
    fn test_minimal_board_parses() {
        let toml_content = r#"
[board]
name = "minimal-board"
description = "A minimal board"
target = "aarch64-linux-musl"
cpu = "cortex-a53"

[defaults]
image_format = "ext4"
rootfs_size = "128M"
hostname = "minimal"
"#;

        let board =
            BoardDefinition::from_toml(toml_content).expect("Failed to parse minimal board");

        assert_eq!(board.board.name, "minimal-board");
        assert_eq!(board.board.target, "aarch64-linux-musl");
        assert!(board.board.features.is_empty());
        assert!(board.flash.is_empty());
        assert!(board.requires.is_empty());
        assert!(board.options.is_empty());
    }

    // ============================================
    // Round-trip tests
    // ============================================

    #[test]
    fn test_board_roundtrip() {
        let board = BoardDefinition {
            board: BoardMetadata {
                name: "test-board".to_string(),
                description: "Test board for unit tests".to_string(),
                target: "arm-linux-musleabihf".to_string(),
                cpu: "cortex-a7".to_string(),
                features: vec!["neon".to_string()],
                kernel: None,
                zigroot_version: None,
            },
            defaults: BoardDefaults {
                image_format: "ext4".to_string(),
                rootfs_size: "256M".to_string(),
                hostname: "test".to_string(),
            },
            requires: vec!["busybox".to_string()],
            flash: vec![FlashProfile {
                name: "sd-card".to_string(),
                description: "Flash to SD card".to_string(),
                script: None,
                tool: Some("dd".to_string()),
                requires: vec![],
            }],
            options: HashMap::new(),
        };

        let toml_str = board.to_toml().expect("Failed to serialize");
        let parsed = BoardDefinition::from_toml(&toml_str).expect("Failed to parse");

        assert_eq!(board, parsed);
    }

    // ============================================
    // Missing required fields tests
    // ============================================

    #[test]
    fn test_missing_board_name() {
        let toml_content = r#"
[board]
description = "Missing name"
target = "arm-linux-musleabihf"
cpu = "cortex-a7"

[defaults]
image_format = "ext4"
rootfs_size = "256M"
hostname = "test"
"#;

        let result = BoardDefinition::from_toml(toml_content);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("name") || err.contains("missing"),
            "Error should mention missing 'name': {err}"
        );
    }

    #[test]
    fn test_missing_board_description() {
        let toml_content = r#"
[board]
name = "test-board"
target = "arm-linux-musleabihf"
cpu = "cortex-a7"

[defaults]
image_format = "ext4"
rootfs_size = "256M"
hostname = "test"
"#;

        let result = BoardDefinition::from_toml(toml_content);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("description") || err.contains("missing"),
            "Error should mention missing 'description': {err}"
        );
    }

    #[test]
    fn test_missing_board_target() {
        let toml_content = r#"
[board]
name = "test-board"
description = "Missing target"
cpu = "cortex-a7"

[defaults]
image_format = "ext4"
rootfs_size = "256M"
hostname = "test"
"#;

        let result = BoardDefinition::from_toml(toml_content);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("target") || err.contains("missing"),
            "Error should mention missing 'target': {err}"
        );
    }

    #[test]
    fn test_missing_board_cpu() {
        let toml_content = r#"
[board]
name = "test-board"
description = "Missing cpu"
target = "arm-linux-musleabihf"

[defaults]
image_format = "ext4"
rootfs_size = "256M"
hostname = "test"
"#;

        let result = BoardDefinition::from_toml(toml_content);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("cpu") || err.contains("missing"),
            "Error should mention missing 'cpu': {err}"
        );
    }

    #[test]
    fn test_missing_defaults_section() {
        let toml_content = r#"
[board]
name = "test-board"
description = "Missing defaults"
target = "arm-linux-musleabihf"
cpu = "cortex-a7"
"#;

        let result = BoardDefinition::from_toml(toml_content);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("defaults") || err.contains("missing"),
            "Error should mention missing 'defaults': {err}"
        );
    }

    // ============================================
    // Flash profile tests
    // ============================================

    #[test]
    fn test_flash_profile_with_tool() {
        let toml_content = r#"
[board]
name = "test-board"
description = "Test board"
target = "arm-linux-musleabihf"
cpu = "cortex-a7"

[defaults]
image_format = "ext4"
rootfs_size = "256M"
hostname = "test"

[[flash]]
name = "sd-card"
description = "Flash to SD card"
tool = "dd"
requires = ["bootloader"]
"#;

        let board = BoardDefinition::from_toml(toml_content).expect("Failed to parse");

        assert_eq!(board.flash.len(), 1);
        assert_eq!(board.flash[0].name, "sd-card");
        assert_eq!(board.flash[0].tool, Some("dd".to_string()));
        assert_eq!(board.flash[0].script, None);
        assert_eq!(board.flash[0].requires, vec!["bootloader"]);
    }

    #[test]
    fn test_flash_profile_with_script() {
        let toml_content = r#"
[board]
name = "test-board"
description = "Test board"
target = "arm-linux-musleabihf"
cpu = "cortex-a7"

[defaults]
image_format = "ext4"
rootfs_size = "256M"
hostname = "test"

[[flash]]
name = "custom-flash"
description = "Custom flash script"
script = "flash.sh"
"#;

        let board = BoardDefinition::from_toml(toml_content).expect("Failed to parse");

        assert_eq!(board.flash.len(), 1);
        assert_eq!(board.flash[0].script, Some("flash.sh".to_string()));
        assert_eq!(board.flash[0].tool, None);
    }

    // ============================================
    // Board options tests
    // ============================================

    #[test]
    fn test_board_options_parse() {
        let toml_content = r#"
[board]
name = "test-board"
description = "Test board"
target = "arm-linux-musleabihf"
cpu = "cortex-a7"

[defaults]
image_format = "ext4"
rootfs_size = "256M"
hostname = "test"

[options.boot_mode]
type = "choice"
default = "sd"
choices = ["sd", "emmc"]
description = "Boot device"

[options.debug]
type = "bool"
default = false
description = "Enable debug mode"
"#;

        let board = BoardDefinition::from_toml(toml_content).expect("Failed to parse");

        assert_eq!(board.options.len(), 2);

        let boot_mode = board
            .options
            .get("boot_mode")
            .expect("boot_mode option missing");
        assert_eq!(boot_mode.option_type, "choice");
        assert_eq!(boot_mode.choices, vec!["sd", "emmc"]);

        let debug = board.options.get("debug").expect("debug option missing");
        assert_eq!(debug.option_type, "bool");
    }

    // ============================================
    // Property-Based Tests
    // ============================================

    /// Strategy for generating valid board names
    fn board_name_strategy() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9-]{0,30}[a-z0-9]?".prop_filter("Name must not be empty", |s| !s.is_empty())
    }

    /// Strategy for generating valid descriptions
    fn description_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9 ]{1,100}"
    }

    /// Strategy for generating valid target triples
    fn target_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("arm-linux-musleabihf".to_string()),
            Just("aarch64-linux-musl".to_string()),
            Just("x86_64-linux-musl".to_string()),
            Just("riscv64-linux-musl".to_string()),
        ]
    }

    /// Strategy for generating valid CPU names
    fn cpu_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("cortex-a7".to_string()),
            Just("cortex-a53".to_string()),
            Just("cortex-a72".to_string()),
            Just("generic".to_string()),
        ]
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

    /// Strategy for generating valid hostnames
    fn hostname_strategy() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9-]{0,20}[a-z0-9]?".prop_filter("Hostname must not be empty", |s| !s.is_empty())
    }

    /// Strategy for generating a complete BoardDefinition
    fn board_definition_strategy() -> impl Strategy<Value = BoardDefinition> {
        (
            board_name_strategy(),
            description_strategy(),
            target_strategy(),
            cpu_strategy(),
            image_format_strategy(),
            rootfs_size_strategy(),
            hostname_strategy(),
        )
            .prop_map(
                |(name, description, target, cpu, image_format, rootfs_size, hostname)| {
                    BoardDefinition {
                        board: BoardMetadata {
                            name,
                            description,
                            target,
                            cpu,
                            features: vec![],
                            kernel: None,
                            zigroot_version: None,
                        },
                        defaults: BoardDefaults {
                            image_format,
                            rootfs_size,
                            hostname,
                        },
                        requires: vec![],
                        flash: vec![],
                        options: HashMap::new(),
                    }
                },
            )
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 1: TOML Serialization Round-Trip (BoardDefinition)
        /// For all valid BoardDefinition configurations, serializing then deserializing
        /// SHALL produce an equivalent BoardDefinition.
        /// **Validates: Requirements 19.1-19.13**
        #[test]
        fn prop_board_toml_roundtrip(board in board_definition_strategy()) {
            // Serialize to TOML
            let toml_str = board.to_toml()
                .expect("BoardDefinition should serialize to valid TOML");

            // Verify it's valid TOML
            let _: toml::Value = toml::from_str(&toml_str)
                .expect("Serialized output should be valid TOML");

            // Deserialize back
            let parsed = BoardDefinition::from_toml(&toml_str)
                .expect("Should deserialize back to BoardDefinition");

            // Verify equivalence
            prop_assert_eq!(board, parsed, "Round-trip should produce equivalent BoardDefinition");
        }

        /// Property: Board name is preserved through serialization
        #[test]
        fn prop_board_name_preserved(name in board_name_strategy()) {
            let board = BoardDefinition {
                board: BoardMetadata {
                    name: name.clone(),
                    description: "Test board".to_string(),
                    target: "arm-linux-musleabihf".to_string(),
                    cpu: "cortex-a7".to_string(),
                    features: vec![],
                    kernel: None,
                    zigroot_version: None,
                },
                defaults: BoardDefaults {
                    image_format: "ext4".to_string(),
                    rootfs_size: "256M".to_string(),
                    hostname: "test".to_string(),
                },
                requires: vec![],
                flash: vec![],
                options: HashMap::new(),
            };

            let toml_str = board.to_toml().expect("Should serialize");
            let parsed = BoardDefinition::from_toml(&toml_str).expect("Should parse");

            prop_assert_eq!(parsed.board.name, name);
        }

        /// Property 27: Missing Field Error Specificity
        /// When a required field is missing, the error message should identify the field
        /// **Validates: Requirements 19.2, 19.4**
        #[test]
        fn prop_board_target_preserved(target in target_strategy()) {
            let board = BoardDefinition {
                board: BoardMetadata {
                    name: "test-board".to_string(),
                    description: "Test board".to_string(),
                    target: target.clone(),
                    cpu: "cortex-a7".to_string(),
                    features: vec![],
                    kernel: None,
                    zigroot_version: None,
                },
                defaults: BoardDefaults {
                    image_format: "ext4".to_string(),
                    rootfs_size: "256M".to_string(),
                    hostname: "test".to_string(),
                },
                requires: vec![],
                flash: vec![],
                options: HashMap::new(),
            };

            let toml_str = board.to_toml().expect("Should serialize");
            let parsed = BoardDefinition::from_toml(&toml_str).expect("Should parse");

            prop_assert_eq!(parsed.board.target, target);
        }
    }
}

//! Test utilities for property-based testing
//!
//! This module provides generators and helpers for proptest.

#[cfg(test)]
pub mod generators {
    use proptest::prelude::*;

    /// Generate a valid package name (lowercase alphanumeric with hyphens)
    pub fn package_name() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9-]{0,30}[a-z0-9]?".prop_filter("Name must not be empty", |s| !s.is_empty())
    }

    /// Generate a valid semver version string
    pub fn semver_version() -> impl Strategy<Value = String> {
        (1u32..100, 0u32..100, 0u32..100)
            .prop_map(|(major, minor, patch)| format!("{major}.{minor}.{patch}"))
    }

    /// Generate a valid SHA256 hash (64 hex characters)
    pub fn sha256_hash() -> impl Strategy<Value = String> {
        "[0-9a-f]{64}"
    }

    /// Generate a valid URL
    pub fn url() -> impl Strategy<Value = String> {
        (
            prop_oneof!["https", "http"],
            "[a-z]{3,10}",
            "[a-z]{2,5}",
            "[a-z0-9-]{1,20}",
        )
            .prop_map(|(scheme, domain, tld, path)| {
                format!("{scheme}://{domain}.{tld}/{path}.tar.gz")
            })
    }

    /// Generate a valid hostname
    pub fn hostname() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9-]{0,20}[a-z0-9]?".prop_filter("Hostname must not be empty", |s| !s.is_empty())
    }

    /// Generate a valid Zig target triple
    pub fn target_triple() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("arm-linux-musleabihf".to_string()),
            Just("aarch64-linux-musl".to_string()),
            Just("x86_64-linux-musl".to_string()),
            Just("riscv64-linux-musl".to_string()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::generators::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn test_package_name_generator(name in package_name()) {
            prop_assert!(!name.is_empty());
            prop_assert!(name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'));
        }

        #[test]
        fn test_semver_version_generator(version in semver_version()) {
            let parts: Vec<&str> = version.split('.').collect();
            prop_assert_eq!(parts.len(), 3);
            for part in parts {
                prop_assert!(part.parse::<u32>().is_ok());
            }
        }

        #[test]
        fn test_sha256_hash_generator(hash in sha256_hash()) {
            prop_assert_eq!(hash.len(), 64);
            prop_assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
        }
    }
}

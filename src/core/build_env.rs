//! Build environment setup
//!
//! Provides build environment configuration for package compilation.
//! Sets up environment variables like CC, TARGET, JOBS, SRCDIR, DESTDIR, PREFIX.
//!
//! **Validates: Requirements 18.17-18.27**

use std::collections::HashMap;
use std::path::PathBuf;

/// Build environment for a package.
///
/// For Zig-based builds (the default), cross-compilation is handled internally
/// by Zig - we just set CC="zig cc -target <target>" and Zig handles everything.
/// No sysroot, no separate AR/LD/RANLIB needed.
///
/// For GCC-based builds (kernel, bootloader), we set up a traditional
/// cross-compilation environment with the downloaded toolchain.
#[derive(Debug, Clone, PartialEq)]
pub struct BuildEnvironment {
    /// Compiler command (e.g., "zig cc -target arm-linux-musleabihf" or "arm-linux-gnueabihf-gcc")
    pub cc: String,
    /// C++ compiler (e.g., "zig c++ -target arm-linux-musleabihf" or "arm-linux-gnueabihf-g++")
    pub cxx: String,
    /// Archiver (None for Zig, Some for GCC)
    pub ar: Option<String>,
    /// Target triple (e.g., "arm-linux-musleabihf")
    pub target: String,
    /// CPU type (e.g., "cortex-a7")
    pub cpu: String,
    /// Source directory (extracted package sources)
    pub srcdir: PathBuf,
    /// Destination directory for installed files (staging area)
    pub destdir: PathBuf,
    /// Install prefix (usually /usr)
    pub prefix: String,
    /// Number of parallel jobs
    pub jobs: usize,
    /// Additional environment variables
    pub extra_env: HashMap<String, String>,
}

impl BuildEnvironment {
    /// Create environment for Zig-based build (default)
    pub fn for_zig(target: &str, cpu: &str, srcdir: PathBuf, destdir: PathBuf) -> Self {
        Self {
            cc: format!("zig cc -target {target}"),
            cxx: format!("zig c++ -target {target}"),
            ar: None, // Zig handles archiving internally
            target: target.to_string(),
            cpu: cpu.to_string(),
            srcdir,
            destdir,
            prefix: "/usr".to_string(),
            jobs: num_cpus::get(),
            extra_env: HashMap::new(),
        }
    }

    /// Create environment for GCC-based build (kernel, bootloader)
    pub fn for_gcc(
        toolchain_prefix: &str,
        target: &str,
        cpu: &str,
        srcdir: PathBuf,
        destdir: PathBuf,
    ) -> Self {
        Self {
            cc: format!("{toolchain_prefix}gcc"),
            cxx: format!("{toolchain_prefix}g++"),
            ar: Some(format!("{toolchain_prefix}ar")),
            target: target.to_string(),
            cpu: cpu.to_string(),
            srcdir,
            destdir,
            prefix: "/usr".to_string(),
            jobs: num_cpus::get(),
            extra_env: HashMap::new(),
        }
    }

    /// Set the number of parallel jobs
    #[must_use]
    pub fn with_jobs(mut self, jobs: usize) -> Self {
        self.jobs = jobs;
        self
    }

    /// Set the install prefix
    #[must_use]
    pub fn with_prefix(mut self, prefix: &str) -> Self {
        self.prefix = prefix.to_string();
        self
    }

    /// Add an extra environment variable
    #[must_use]
    pub fn with_env(mut self, key: &str, value: &str) -> Self {
        self.extra_env.insert(key.to_string(), value.to_string());
        self
    }

    /// Convert to environment variable map for process execution
    pub fn to_env_map(&self) -> HashMap<String, String> {
        let mut env = HashMap::new();

        // Core build variables
        env.insert("CC".to_string(), self.cc.clone());
        env.insert("CXX".to_string(), self.cxx.clone());
        env.insert("TARGET".to_string(), self.target.clone());
        env.insert("CPU".to_string(), self.cpu.clone());
        env.insert("SRCDIR".to_string(), self.srcdir.display().to_string());
        env.insert("DESTDIR".to_string(), self.destdir.display().to_string());
        env.insert("PREFIX".to_string(), self.prefix.clone());
        env.insert("JOBS".to_string(), self.jobs.to_string());

        // Optional archiver
        if let Some(ref ar) = self.ar {
            env.insert("AR".to_string(), ar.clone());
        }

        // Extra environment variables
        for (key, value) in &self.extra_env {
            env.insert(key.clone(), value.clone());
        }

        env
    }

    /// Check if all required environment variables are set
    pub fn validate(&self) -> Result<(), BuildEnvError> {
        if self.cc.is_empty() {
            return Err(BuildEnvError::MissingVariable("CC".to_string()));
        }
        if self.target.is_empty() {
            return Err(BuildEnvError::MissingVariable("TARGET".to_string()));
        }
        if self.jobs == 0 {
            return Err(BuildEnvError::InvalidValue {
                variable: "JOBS".to_string(),
                reason: "must be greater than 0".to_string(),
            });
        }
        Ok(())
    }
}

/// Build environment errors
#[derive(Debug, Clone, PartialEq)]
pub enum BuildEnvError {
    /// Required variable is missing
    MissingVariable(String),
    /// Variable has invalid value
    InvalidValue { variable: String, reason: String },
}

impl std::fmt::Display for BuildEnvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingVariable(var) => write!(f, "Missing required environment variable: {var}"),
            Self::InvalidValue { variable, reason } => {
                write!(f, "Invalid value for {variable}: {reason}")
            }
        }
    }
}

impl std::error::Error for BuildEnvError {}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // ============================================
    // Unit Tests
    // ============================================

    #[test]
    fn test_zig_environment_creation() {
        let env = BuildEnvironment::for_zig(
            "arm-linux-musleabihf",
            "cortex-a7",
            PathBuf::from("/src"),
            PathBuf::from("/dest"),
        );

        assert_eq!(env.cc, "zig cc -target arm-linux-musleabihf");
        assert_eq!(env.cxx, "zig c++ -target arm-linux-musleabihf");
        assert!(env.ar.is_none());
        assert_eq!(env.target, "arm-linux-musleabihf");
        assert_eq!(env.cpu, "cortex-a7");
        assert_eq!(env.prefix, "/usr");
    }

    #[test]
    fn test_gcc_environment_creation() {
        let env = BuildEnvironment::for_gcc(
            "arm-linux-gnueabihf-",
            "arm-linux-gnueabihf",
            "cortex-a7",
            PathBuf::from("/src"),
            PathBuf::from("/dest"),
        );

        assert_eq!(env.cc, "arm-linux-gnueabihf-gcc");
        assert_eq!(env.cxx, "arm-linux-gnueabihf-g++");
        assert_eq!(env.ar, Some("arm-linux-gnueabihf-ar".to_string()));
        assert_eq!(env.target, "arm-linux-gnueabihf");
    }

    #[test]
    fn test_env_map_contains_required_variables() {
        let env = BuildEnvironment::for_zig(
            "aarch64-linux-musl",
            "cortex-a53",
            PathBuf::from("/build/src"),
            PathBuf::from("/build/dest"),
        )
        .with_jobs(4);

        let map = env.to_env_map();

        assert!(map.contains_key("CC"));
        assert!(map.contains_key("CXX"));
        assert!(map.contains_key("TARGET"));
        assert!(map.contains_key("CPU"));
        assert!(map.contains_key("SRCDIR"));
        assert!(map.contains_key("DESTDIR"));
        assert!(map.contains_key("PREFIX"));
        assert!(map.contains_key("JOBS"));

        assert_eq!(map.get("TARGET").unwrap(), "aarch64-linux-musl");
        assert_eq!(map.get("JOBS").unwrap(), "4");
    }

    #[test]
    fn test_extra_env_variables() {
        let env = BuildEnvironment::for_zig(
            "arm-linux-musleabihf",
            "cortex-a7",
            PathBuf::from("/src"),
            PathBuf::from("/dest"),
        )
        .with_env("CUSTOM_VAR", "custom_value")
        .with_env("ANOTHER_VAR", "another_value");

        let map = env.to_env_map();

        assert_eq!(map.get("CUSTOM_VAR").unwrap(), "custom_value");
        assert_eq!(map.get("ANOTHER_VAR").unwrap(), "another_value");
    }

    #[test]
    fn test_validation_passes_for_valid_env() {
        let env = BuildEnvironment::for_zig(
            "arm-linux-musleabihf",
            "cortex-a7",
            PathBuf::from("/src"),
            PathBuf::from("/dest"),
        );

        assert!(env.validate().is_ok());
    }

    #[test]
    fn test_validation_fails_for_empty_cc() {
        let mut env = BuildEnvironment::for_zig(
            "arm-linux-musleabihf",
            "cortex-a7",
            PathBuf::from("/src"),
            PathBuf::from("/dest"),
        );
        env.cc = String::new();

        assert!(matches!(
            env.validate(),
            Err(BuildEnvError::MissingVariable(_))
        ));
    }

    #[test]
    fn test_validation_fails_for_zero_jobs() {
        let mut env = BuildEnvironment::for_zig(
            "arm-linux-musleabihf",
            "cortex-a7",
            PathBuf::from("/src"),
            PathBuf::from("/dest"),
        );
        env.jobs = 0;

        assert!(matches!(
            env.validate(),
            Err(BuildEnvError::InvalidValue { .. })
        ));
    }

    // ============================================
    // Property-Based Tests
    // ============================================

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

    /// Strategy for generating valid paths
    fn path_strategy() -> impl Strategy<Value = PathBuf> {
        "[a-z]{1,10}(/[a-z]{1,10}){0,3}".prop_map(PathBuf::from)
    }

    /// Strategy for generating valid job counts
    fn jobs_strategy() -> impl Strategy<Value = usize> {
        1usize..=32
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 14: Build Environment Variables
        /// For any package build, the environment SHALL contain correctly set variables
        /// for CC, TARGET, JOBS, SRCDIR, DESTDIR, PREFIX, and all dependency paths.
        /// **Validates: Requirements 18.20, 18.33**
        #[test]
        fn prop_build_environment_contains_required_variables(
            target in target_strategy(),
            cpu in cpu_strategy(),
            srcdir in path_strategy(),
            destdir in path_strategy(),
            jobs in jobs_strategy(),
        ) {
            let env = BuildEnvironment::for_zig(&target, &cpu, srcdir.clone(), destdir.clone())
                .with_jobs(jobs);

            let map = env.to_env_map();

            // Verify all required variables are present
            prop_assert!(map.contains_key("CC"), "CC must be present");
            prop_assert!(map.contains_key("CXX"), "CXX must be present");
            prop_assert!(map.contains_key("TARGET"), "TARGET must be present");
            prop_assert!(map.contains_key("CPU"), "CPU must be present");
            prop_assert!(map.contains_key("SRCDIR"), "SRCDIR must be present");
            prop_assert!(map.contains_key("DESTDIR"), "DESTDIR must be present");
            prop_assert!(map.contains_key("PREFIX"), "PREFIX must be present");
            prop_assert!(map.contains_key("JOBS"), "JOBS must be present");

            // Verify values are correct
            prop_assert_eq!(map.get("TARGET").unwrap(), &target);
            prop_assert_eq!(map.get("CPU").unwrap(), &cpu);
            prop_assert_eq!(map.get("SRCDIR").unwrap(), &srcdir.display().to_string());
            prop_assert_eq!(map.get("DESTDIR").unwrap(), &destdir.display().to_string());
            prop_assert_eq!(map.get("JOBS").unwrap(), &jobs.to_string());

            // Verify CC contains target
            prop_assert!(
                map.get("CC").unwrap().contains(&target),
                "CC should contain target triple"
            );

            // Verify environment validates
            prop_assert!(env.validate().is_ok(), "Environment should be valid");
        }

        /// Property: GCC environment also contains required variables
        #[test]
        fn prop_gcc_environment_contains_required_variables(
            target in target_strategy(),
            cpu in cpu_strategy(),
            srcdir in path_strategy(),
            destdir in path_strategy(),
            jobs in jobs_strategy(),
        ) {
            let toolchain_prefix = format!("{target}-");
            let env = BuildEnvironment::for_gcc(&toolchain_prefix, &target, &cpu, srcdir.clone(), destdir.clone())
                .with_jobs(jobs);

            let map = env.to_env_map();

            // Verify all required variables are present
            prop_assert!(map.contains_key("CC"), "CC must be present");
            prop_assert!(map.contains_key("CXX"), "CXX must be present");
            prop_assert!(map.contains_key("AR"), "AR must be present for GCC");
            prop_assert!(map.contains_key("TARGET"), "TARGET must be present");
            prop_assert!(map.contains_key("SRCDIR"), "SRCDIR must be present");
            prop_assert!(map.contains_key("DESTDIR"), "DESTDIR must be present");
            prop_assert!(map.contains_key("PREFIX"), "PREFIX must be present");
            prop_assert!(map.contains_key("JOBS"), "JOBS must be present");

            // Verify environment validates
            prop_assert!(env.validate().is_ok(), "Environment should be valid");
        }

        /// Property: Extra environment variables are preserved
        #[test]
        fn prop_extra_env_preserved(
            target in target_strategy(),
            cpu in cpu_strategy(),
            key in "[A-Z_]{1,10}",
            value in "[a-zA-Z0-9_]{1,20}",
        ) {
            let env = BuildEnvironment::for_zig(&target, &cpu, PathBuf::from("/src"), PathBuf::from("/dest"))
                .with_env(&key, &value);

            let map = env.to_env_map();

            prop_assert_eq!(
                map.get(&key).unwrap(),
                &value,
                "Extra env variable should be preserved"
            );
        }
    }
}

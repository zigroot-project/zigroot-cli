//! Default configuration values

/// Maximum number of download retry attempts
pub const MAX_DOWNLOAD_RETRIES: u32 = 3;

/// Default number of parallel downloads
pub const DEFAULT_PARALLEL_DOWNLOADS: usize = 4;

/// Default number of parallel build jobs
pub const DEFAULT_BUILD_JOBS: usize = 4;

/// Default image format
pub const DEFAULT_IMAGE_FORMAT: &str = "ext4";

/// Default rootfs size
pub const DEFAULT_ROOTFS_SIZE: &str = "256M";

/// Default hostname
pub const DEFAULT_HOSTNAME: &str = "zigroot";

/// Cache TTL for registry index (in seconds)
pub const REGISTRY_CACHE_TTL: u64 = 3600; // 1 hour

/// Minimum proptest iterations
pub const MIN_PROPTEST_ITERATIONS: u32 = 100;

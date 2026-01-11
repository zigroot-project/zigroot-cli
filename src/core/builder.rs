//! Build orchestration logic
//!
//! Coordinates the build process across multiple packages.

/// Build orchestrator state
#[derive(Debug, Default)]
pub struct BuildOrchestrator {
    /// Packages to build
    packages: Vec<String>,
    /// Build order (computed from dependency graph)
    build_order: Vec<String>,
}

impl BuildOrchestrator {
    /// Create a new build orchestrator
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the packages to build
    #[must_use]
    pub fn with_packages(mut self, packages: Vec<String>) -> Self {
        self.packages = packages;
        self
    }

    /// Set the build order
    #[must_use]
    pub fn with_build_order(mut self, order: Vec<String>) -> Self {
        self.build_order = order;
        self
    }

    /// Get the build order
    pub fn build_order(&self) -> &[String] {
        &self.build_order
    }
}

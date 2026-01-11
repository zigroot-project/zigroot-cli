//! Zigroot - Modern embedded Linux rootfs builder
//!
//! This library provides the core functionality for building embedded Linux
//! root filesystems using Zig's cross-compilation capabilities.
//!
//! # Architecture
//!
//! The crate is organized into several modules:
//!
//! - [`cli`] - Command-line interface parsing and output formatting
//! - [`core`] - Business logic (no I/O operations)
//! - [`registry`] - Package and board registry client
//! - [`infra`] - Infrastructure layer (network, filesystem, processes)
//! - [`config`] - Configuration and constants
//! - [`error`] - Error types and handling

pub mod cli;
pub mod config;
pub mod core;
pub mod error;
pub mod infra;
pub mod registry;

#[cfg(test)]
pub mod test_utils;

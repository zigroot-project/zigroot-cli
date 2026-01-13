//! Core business logic module
//!
//! This module contains all business logic for zigroot.
//! It has NO I/O operations - those belong in [`crate::infra`].
//!
//! # Submodules
//!
//! - [`manifest`] - Manifest (zigroot.toml) parsing and validation
//! - [`package`] - Package definition handling
//! - [`board`] - Board definition handling
//! - [`resolver`] - Dependency resolution
//! - [`builder`] - Build orchestration logic
//! - [`build_env`] - Build environment setup
//! - [`lock`] - Lock file handling
//! - [`init`] - Project initialization logic
//! - [`add`] - Package addition logic
//! - [`remove`] - Package removal logic
//! - [`update`] - Package update logic
//! - [`fetch`] - Package fetch logic
//! - [`clean`] - Clean build artifacts logic
//! - [`check`] - Configuration validation logic
//! - [`search`] - Search functionality for packages and boards
//! - [`flash`] - Device flashing logic
//! - [`external`] - External artifact management
//! - [`compress`] - Binary compression using UPX
//! - [`kernel`] - Linux kernel build support
//! - [`global_config`] - Global configuration management
//! - [`shared_storage`] - Shared downloads and build cache

pub mod add;
pub mod board;
pub mod build_env;
pub mod builder;
pub mod cache;
pub mod check;
pub mod clean;
pub mod compress;
pub mod config;
pub mod doctor;
pub mod external;
pub mod fetch;
pub mod flash;
pub mod global_config;
pub mod init;
pub mod kernel;
pub mod license;
pub mod lock;
pub mod manifest;
pub mod options;
pub mod package;
pub mod remove;
pub mod resolver;
pub mod sdk;
pub mod search;
pub mod shared_storage;
pub mod tree;
pub mod update;
pub mod version;

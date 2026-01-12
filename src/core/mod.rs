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

pub mod add;
pub mod board;
pub mod build_env;
pub mod builder;
pub mod clean;
pub mod fetch;
pub mod init;
pub mod lock;
pub mod manifest;
pub mod options;
pub mod package;
pub mod remove;
pub mod resolver;
pub mod update;

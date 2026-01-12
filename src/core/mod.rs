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
//! - [`lock`] - Lock file handling
//! - [`init`] - Project initialization logic
//! - [`add`] - Package addition logic
//! - [`remove`] - Package removal logic

pub mod add;
pub mod board;
pub mod builder;
pub mod init;
pub mod lock;
pub mod manifest;
pub mod options;
pub mod package;
pub mod remove;
pub mod resolver;

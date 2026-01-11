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

pub mod board;
pub mod builder;
pub mod lock;
pub mod manifest;
pub mod options;
pub mod package;
pub mod resolver;

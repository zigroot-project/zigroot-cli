//! Infrastructure layer
//!
//! Handles all I/O operations: network, filesystem, and external processes.
//! This module is the only place where side effects occur.

pub mod download;
pub mod filesystem;
pub mod gcc_toolchain;
pub mod git;
pub mod toolchain;

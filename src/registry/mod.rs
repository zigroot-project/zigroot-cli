//! Package and board registry client
//!
//! Handles fetching package and board definitions from GitHub-hosted registries.

pub mod cache;
pub mod client;

pub use client::RegistryClient;

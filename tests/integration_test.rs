//! Integration tests for zigroot CLI
//!
//! These tests verify end-to-end functionality of the CLI commands.

mod common;

use common::TestProject;

#[test]
fn test_project_creation() {
    let project = TestProject::new();
    assert!(project.path().exists());
}

#[test]
fn test_file_creation() {
    let project = TestProject::new();
    project.create_file("test.txt", "hello world");
    assert!(project.file_exists("test.txt"));
    assert_eq!(project.read_file("test.txt"), "hello world");
}

#[test]
fn test_directory_creation() {
    let project = TestProject::new();
    project.create_dir("subdir/nested");
    assert!(project.path().join("subdir/nested").exists());
}

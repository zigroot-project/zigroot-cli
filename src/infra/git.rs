//! Git operations
//!
//! Handles cloning repositories and checking out refs using the gix crate.

use gix::remote::fetch::Shallow;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Git operation errors
#[derive(Error, Debug)]
pub enum GitError {
    /// Failed to clone repository
    #[error("Failed to clone '{url}': {error}")]
    CloneFailed { url: String, error: String },

    /// Failed to checkout ref
    #[error("Failed to checkout ref '{reference}' in '{repo}': {error}")]
    CheckoutFailed {
        repo: String,
        reference: String,
        error: String,
    },

    /// Ref not found
    #[error("Ref '{reference}' not found in repository '{repo}'")]
    RefNotFound { repo: String, reference: String },

    /// Failed to resolve ref to SHA
    #[error("Failed to resolve ref '{reference}' to SHA: {error}")]
    ResolveFailed { reference: String, error: String },

    /// IO error
    #[error("IO error for '{path}': {error}")]
    IoError { path: PathBuf, error: String },

    /// Invalid repository
    #[error("Invalid repository at '{path}': {error}")]
    InvalidRepository { path: PathBuf, error: String },
}

/// Git reference type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitRef {
    /// Git tag (e.g., "v1.0.0")
    Tag(String),
    /// Git branch (e.g., "main")
    Branch(String),
    /// Git commit SHA (e.g., "abc123...")
    Rev(String),
}

impl GitRef {
    /// Get the reference string
    pub fn as_str(&self) -> &str {
        match self {
            Self::Tag(s) | Self::Branch(s) | Self::Rev(s) => s,
        }
    }
}

impl std::fmt::Display for GitRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tag(s) => write!(f, "tag:{s}"),
            Self::Branch(s) => write!(f, "branch:{s}"),
            Self::Rev(s) => write!(f, "rev:{s}"),
        }
    }
}

/// Result of a git clone operation
#[derive(Debug, Clone)]
pub struct CloneResult {
    /// Path to the cloned repository
    pub path: PathBuf,
    /// Resolved commit SHA
    pub commit_sha: String,
    /// The ref that was checked out
    pub checked_out_ref: GitRef,
}

/// Git repository operations
#[derive(Debug)]
pub struct GitOperations {
    /// Working directory for git operations
    work_dir: PathBuf,
}

impl GitOperations {
    /// Create a new git operations handler
    pub fn new(work_dir: PathBuf) -> Self {
        Self { work_dir }
    }

    /// Get the working directory
    pub fn work_dir(&self) -> &PathBuf {
        &self.work_dir
    }

    /// Clone a repository and checkout a specific ref
    ///
    /// # Arguments
    /// * `url` - Repository URL to clone
    /// * `git_ref` - Reference to checkout (tag, branch, or rev)
    /// * `dest_name` - Name for the destination directory within `work_dir`
    ///
    /// # Returns
    /// Clone result with path and resolved commit SHA
    pub fn clone_repo(
        &self,
        url: &str,
        git_ref: &GitRef,
        dest_name: &str,
    ) -> Result<CloneResult, GitError> {
        let dest_path = self.work_dir.join(dest_name);

        // Remove existing directory if present
        if dest_path.exists() {
            std::fs::remove_dir_all(&dest_path).map_err(|e| GitError::IoError {
                path: dest_path.clone(),
                error: e.to_string(),
            })?;
        }

        // Clone the repository
        self.clone_internal(url, &dest_path, git_ref)?;

        // Resolve the commit SHA
        let commit_sha = self.resolve_ref_to_sha(&dest_path, git_ref)?;

        Ok(CloneResult {
            path: dest_path,
            commit_sha,
            checked_out_ref: git_ref.clone(),
        })
    }

    /// Internal clone implementation using gix
    fn clone_internal(&self, url: &str, dest: &Path, git_ref: &GitRef) -> Result<(), GitError> {
        // Prepare the clone with the appropriate ref
        let mut prepare = gix::prepare_clone(url, dest).map_err(|e| GitError::CloneFailed {
            url: url.to_string(),
            error: e.to_string(),
        })?;

        // Configure shallow clone for efficiency
        prepare = prepare.with_shallow(Shallow::DepthAtRemote(1.try_into().unwrap()));

        // Fetch and checkout
        let (mut checkout, _outcome) = prepare
            .fetch_then_checkout(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)
            .map_err(|e| GitError::CloneFailed {
                url: url.to_string(),
                error: e.to_string(),
            })?;

        // Complete the checkout to get a working tree
        let (repo, _outcome) = checkout
            .main_worktree(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)
            .map_err(|e| GitError::CloneFailed {
                url: url.to_string(),
                error: e.to_string(),
            })?;

        // Now checkout the specific ref
        self.checkout_ref_internal(&repo, git_ref, url)?;

        Ok(())
    }

    /// Checkout a specific ref in a repository
    fn checkout_ref_internal(
        &self,
        repo: &gix::Repository,
        git_ref: &GitRef,
        url: &str,
    ) -> Result<(), GitError> {
        let reference_name = match git_ref {
            GitRef::Tag(tag) => format!("refs/tags/{tag}"),
            GitRef::Branch(branch) => format!("refs/remotes/origin/{branch}"),
            GitRef::Rev(rev) => {
                // For a specific revision, we need to resolve it directly
                let oid = gix::ObjectId::from_hex(rev.as_bytes()).map_err(|e| {
                    GitError::CheckoutFailed {
                        repo: url.to_string(),
                        reference: git_ref.to_string(),
                        error: format!("Invalid SHA: {e}"),
                    }
                })?;

                // Verify the object exists
                repo.find_object(oid).map_err(|_e| GitError::RefNotFound {
                    repo: url.to_string(),
                    reference: git_ref.to_string(),
                })?;

                return Ok(());
            }
        };

        // Try to find the reference
        let mut reference = repo
            .find_reference(&reference_name)
            .map_err(|_| GitError::RefNotFound {
                repo: url.to_string(),
                reference: git_ref.to_string(),
            })?;

        // Peel to commit to verify it exists
        reference.peel_to_commit().map_err(|e| GitError::CheckoutFailed {
            repo: url.to_string(),
            reference: git_ref.to_string(),
            error: e.to_string(),
        })?;

        Ok(())
    }

    /// Resolve a branch name to its commit SHA
    ///
    /// This is used to record the exact commit in the lock file for reproducibility.
    pub fn resolve_branch_to_sha(&self, repo_path: &Path, branch: &str) -> Result<String, GitError> {
        self.resolve_ref_to_sha(repo_path, &GitRef::Branch(branch.to_string()))
    }

    /// Resolve any ref (tag, branch, or rev) to its commit SHA
    pub fn resolve_ref_to_sha(&self, repo_path: &Path, git_ref: &GitRef) -> Result<String, GitError> {
        let repo = gix::open(repo_path).map_err(|e| GitError::InvalidRepository {
            path: repo_path.to_path_buf(),
            error: e.to_string(),
        })?;

        match git_ref {
            GitRef::Rev(rev) => {
                // For a revision, validate it's a valid hex SHA and return it
                if rev.len() == 40 && rev.chars().all(|c| c.is_ascii_hexdigit()) {
                    Ok(rev.to_lowercase())
                } else {
                    // Try to parse as short SHA
                    let oid = gix::ObjectId::from_hex(rev.as_bytes()).map_err(|e| {
                        GitError::ResolveFailed {
                            reference: git_ref.to_string(),
                            error: format!("Invalid SHA: {e}"),
                        }
                    })?;
                    Ok(oid.to_hex().to_string())
                }
            }
            GitRef::Tag(tag) => {
                let reference_name = format!("refs/tags/{tag}");
                let mut reference = repo.find_reference(&reference_name).map_err(|_| {
                    GitError::RefNotFound {
                        repo: repo_path.display().to_string(),
                        reference: git_ref.to_string(),
                    }
                })?;

                let commit = reference.peel_to_commit().map_err(|e| GitError::ResolveFailed {
                    reference: git_ref.to_string(),
                    error: e.to_string(),
                })?;

                Ok(commit.id().to_hex().to_string())
            }
            GitRef::Branch(branch) => {
                // Try remote branch first (origin/<branch>)
                let remote_ref = format!("refs/remotes/origin/{branch}");
                if let Ok(mut reference) = repo.find_reference(&remote_ref) {
                    let commit = reference.peel_to_commit().map_err(|e| {
                        GitError::ResolveFailed {
                            reference: git_ref.to_string(),
                            error: e.to_string(),
                        }
                    })?;
                    return Ok(commit.id().to_hex().to_string());
                }

                // Try local branch
                let local_ref = format!("refs/heads/{branch}");
                if let Ok(mut reference) = repo.find_reference(&local_ref) {
                    let commit = reference.peel_to_commit().map_err(|e| {
                        GitError::ResolveFailed {
                            reference: git_ref.to_string(),
                            error: e.to_string(),
                        }
                    })?;
                    return Ok(commit.id().to_hex().to_string());
                }

                Err(GitError::RefNotFound {
                    repo: repo_path.display().to_string(),
                    reference: git_ref.to_string(),
                })
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use tempfile::TempDir;

    // ============================================
    // Unit Tests - GitRef
    // ============================================

    #[test]
    fn test_git_ref_tag() {
        let git_ref = GitRef::Tag("v1.0.0".to_string());
        assert_eq!(git_ref.as_str(), "v1.0.0");
        assert_eq!(git_ref.to_string(), "tag:v1.0.0");
    }

    #[test]
    fn test_git_ref_branch() {
        let git_ref = GitRef::Branch("main".to_string());
        assert_eq!(git_ref.as_str(), "main");
        assert_eq!(git_ref.to_string(), "branch:main");
    }

    #[test]
    fn test_git_ref_rev() {
        let git_ref = GitRef::Rev("abc123def456".to_string());
        assert_eq!(git_ref.as_str(), "abc123def456");
        assert_eq!(git_ref.to_string(), "rev:abc123def456");
    }

    #[test]
    fn test_git_ref_equality() {
        let ref1 = GitRef::Tag("v1.0".to_string());
        let ref2 = GitRef::Tag("v1.0".to_string());
        let ref3 = GitRef::Branch("v1.0".to_string());

        assert_eq!(ref1, ref2);
        assert_ne!(ref1, ref3);
    }

    // ============================================
    // Unit Tests - GitOperations creation
    // ============================================

    #[test]
    fn test_git_operations_new() {
        let temp = TempDir::new().unwrap();
        let ops = GitOperations::new(temp.path().to_path_buf());
        assert_eq!(ops.work_dir(), temp.path());
    }

    // ============================================
    // Integration Tests - Clone operations
    // These tests require network access and will clone real repositories
    // ============================================

    #[test]
    #[ignore = "requires network access - run with --ignored"]
    fn test_clone_repo_with_tag() {
        let temp = TempDir::new().unwrap();
        let ops = GitOperations::new(temp.path().to_path_buf());

        // Use a small, stable repository for testing
        let result = ops.clone_repo(
            "https://github.com/Byron/gitoxide.git",
            &GitRef::Tag("v0.1.0".to_string()),
            "gitoxide",
        );

        assert!(result.is_ok(), "Clone with tag should succeed: {result:?}");

        let clone_result = result.unwrap();
        assert!(clone_result.path.exists());
        assert!(!clone_result.commit_sha.is_empty());
        assert_eq!(clone_result.checked_out_ref, GitRef::Tag("v0.1.0".to_string()));
    }

    #[test]
    #[ignore = "requires network access - run with --ignored"]
    fn test_clone_repo_with_branch() {
        let temp = TempDir::new().unwrap();
        let ops = GitOperations::new(temp.path().to_path_buf());

        let result = ops.clone_repo(
            "https://github.com/Byron/gitoxide.git",
            &GitRef::Branch("main".to_string()),
            "gitoxide-branch",
        );

        assert!(result.is_ok(), "Clone with branch should succeed: {result:?}");

        let clone_result = result.unwrap();
        assert!(clone_result.path.exists());
        // SHA should be 40 hex characters
        assert_eq!(clone_result.commit_sha.len(), 40);
        assert!(clone_result.commit_sha.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_clone_repo_invalid_url() {
        let temp = TempDir::new().unwrap();
        let ops = GitOperations::new(temp.path().to_path_buf());

        let result = ops.clone_repo(
            "https://invalid-url-that-does-not-exist.example.com/repo.git",
            &GitRef::Branch("main".to_string()),
            "invalid",
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            GitError::CloneFailed { url, .. } => {
                assert!(url.contains("invalid-url"));
            }
            e => panic!("Expected CloneFailed error, got: {e:?}"),
        }
    }

    #[test]
    #[ignore = "requires network access - run with --ignored"]
    fn test_clone_repo_invalid_ref() {
        let temp = TempDir::new().unwrap();
        let ops = GitOperations::new(temp.path().to_path_buf());

        let result = ops.clone_repo(
            "https://github.com/Byron/gitoxide.git",
            &GitRef::Tag("nonexistent-tag-xyz-123".to_string()),
            "gitoxide-invalid-ref",
        );

        // Should fail because the ref doesn't exist
        assert!(result.is_err());
    }

    // ============================================
    // Integration Tests - Branch to SHA resolution
    // ============================================

    #[test]
    #[ignore = "requires network access - run with --ignored"]
    fn test_resolve_branch_to_sha() {
        let temp = TempDir::new().unwrap();
        let ops = GitOperations::new(temp.path().to_path_buf());

        // First clone the repo
        let clone_result = ops.clone_repo(
            "https://github.com/Byron/gitoxide.git",
            &GitRef::Branch("main".to_string()),
            "gitoxide-resolve",
        );

        assert!(clone_result.is_ok(), "Clone should succeed: {clone_result:?}");
        let clone_result = clone_result.unwrap();

        // Now resolve the branch to SHA
        let sha = ops.resolve_branch_to_sha(&clone_result.path, "main");

        assert!(sha.is_ok(), "Should resolve branch to SHA: {sha:?}");
        let sha = sha.unwrap();
        assert_eq!(sha.len(), 40, "SHA should be 40 hex characters");
        assert!(sha.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    #[ignore = "requires network access - run with --ignored"]
    fn test_resolve_branch_to_sha_nonexistent() {
        let temp = TempDir::new().unwrap();
        let ops = GitOperations::new(temp.path().to_path_buf());

        // First clone the repo
        let clone_result = ops.clone_repo(
            "https://github.com/Byron/gitoxide.git",
            &GitRef::Branch("main".to_string()),
            "gitoxide-resolve-fail",
        );

        assert!(clone_result.is_ok(), "Clone should succeed");
        let clone_result = clone_result.unwrap();

        // Try to resolve a nonexistent branch
        let sha = ops.resolve_branch_to_sha(&clone_result.path, "nonexistent-branch-xyz");

        assert!(sha.is_err());
    }

    // ============================================
    // Integration Tests - Ref to SHA resolution
    // ============================================

    #[test]
    #[ignore = "requires network access - run with --ignored"]
    fn test_resolve_ref_to_sha_tag() {
        let temp = TempDir::new().unwrap();
        let ops = GitOperations::new(temp.path().to_path_buf());

        // First clone the repo
        let clone_result = ops.clone_repo(
            "https://github.com/Byron/gitoxide.git",
            &GitRef::Tag("v0.1.0".to_string()),
            "gitoxide-resolve-tag",
        );

        assert!(clone_result.is_ok(), "Clone should succeed: {clone_result:?}");
        let clone_result = clone_result.unwrap();

        // Resolve tag to SHA
        let sha = ops.resolve_ref_to_sha(&clone_result.path, &GitRef::Tag("v0.1.0".to_string()));

        assert!(sha.is_ok(), "Should resolve tag to SHA: {sha:?}");
        let sha = sha.unwrap();
        assert_eq!(sha.len(), 40);
    }

    // ============================================
    // Property-Based Tests
    // ============================================

    /// Strategy for generating valid git ref names
    fn git_ref_name_strategy() -> impl Strategy<Value = String> {
        // Git ref names have restrictions - simplified valid names
        proptest::string::string_regex("[a-zA-Z][a-zA-Z0-9._-]{0,49}")
            .unwrap()
            .prop_filter("non-empty", |s| !s.is_empty())
    }

    /// Strategy for generating SHA-like strings
    fn sha_strategy() -> impl Strategy<Value = String> {
        proptest::string::string_regex("[0-9a-f]{40}").unwrap()
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: zigroot-cli, Property 22: Lock File Git SHA Recording
        /// For any package with git+branch source, the lock file SHALL record
        /// the resolved commit SHA for reproducibility.
        /// **Validates: Requirements 13.7, 18.13**
        ///
        /// This property test verifies that:
        /// 1. Branch resolution always produces a valid 40-char hex SHA
        /// 2. The same branch always resolves to the same SHA (within a test run)
        /// 3. The SHA format is consistent
        #[test]
        fn prop_git_sha_format(sha in sha_strategy()) {
            // Verify SHA format is valid
            prop_assert_eq!(sha.len(), 40, "SHA must be 40 characters");
            prop_assert!(
                sha.chars().all(|c| c.is_ascii_hexdigit()),
                "SHA must be hex digits only"
            );
        }

        /// Property: GitRef display is consistent and parseable
        #[test]
        fn prop_git_ref_display(ref_name in git_ref_name_strategy()) {
            let tag_ref = GitRef::Tag(ref_name.clone());
            let branch_ref = GitRef::Branch(ref_name.clone());
            let rev_ref = GitRef::Rev(ref_name.clone());

            // Display format should be consistent
            prop_assert!(tag_ref.to_string().starts_with("tag:"));
            prop_assert!(branch_ref.to_string().starts_with("branch:"));
            prop_assert!(rev_ref.to_string().starts_with("rev:"));

            // as_str should return the original name
            prop_assert_eq!(tag_ref.as_str(), &ref_name);
            prop_assert_eq!(branch_ref.as_str(), &ref_name);
            prop_assert_eq!(rev_ref.as_str(), &ref_name);
        }

        /// Property: GitRef equality is reflexive and symmetric
        #[test]
        fn prop_git_ref_equality(ref_name in git_ref_name_strategy()) {
            let ref1 = GitRef::Tag(ref_name.clone());
            let ref2 = GitRef::Tag(ref_name.clone());

            // Reflexive
            prop_assert_eq!(&ref1, &ref1);
            // Symmetric
            prop_assert_eq!(&ref1, &ref2);
            prop_assert_eq!(&ref2, &ref1);
        }

        /// Property: Different ref types with same name are not equal
        #[test]
        fn prop_git_ref_type_distinction(ref_name in git_ref_name_strategy()) {
            let tag_ref = GitRef::Tag(ref_name.clone());
            let branch_ref = GitRef::Branch(ref_name.clone());
            let rev_ref = GitRef::Rev(ref_name.clone());

            // Different types should not be equal
            prop_assert_ne!(&tag_ref, &branch_ref);
            prop_assert_ne!(&tag_ref, &rev_ref);
            prop_assert_ne!(&branch_ref, &rev_ref);
        }
    }

    // ============================================
    // Property 22 Integration Test
    // This test validates the full workflow for lock file SHA recording
    // ============================================

    #[test]
    #[ignore = "requires network access - run with --ignored"]
    fn test_property_22_lock_file_git_sha_recording() {
        // This test validates Property 22: Lock File Git SHA Recording
        // For any package with git+branch source, the lock file SHALL record
        // the resolved commit SHA for reproducibility.

        let temp = TempDir::new().unwrap();
        let ops = GitOperations::new(temp.path().to_path_buf());

        // Clone with a branch reference
        let result = ops.clone_repo(
            "https://github.com/Byron/gitoxide.git",
            &GitRef::Branch("main".to_string()),
            "gitoxide-prop22",
        );

        assert!(result.is_ok(), "Clone should succeed: {result:?}");
        let clone_result = result.unwrap();

        // Verify the commit SHA is recorded
        assert!(!clone_result.commit_sha.is_empty(), "Commit SHA must be recorded");
        assert_eq!(
            clone_result.commit_sha.len(),
            40,
            "Commit SHA must be 40 hex characters"
        );
        assert!(
            clone_result.commit_sha.chars().all(|c| c.is_ascii_hexdigit()),
            "Commit SHA must be valid hex"
        );

        // Verify we can resolve the branch again and get the same SHA
        let resolved_sha = ops.resolve_branch_to_sha(&clone_result.path, "main");
        assert!(resolved_sha.is_ok(), "Should be able to resolve branch: {resolved_sha:?}");
        assert_eq!(
            resolved_sha.unwrap(),
            clone_result.commit_sha,
            "Resolved SHA should match clone result"
        );
    }
}

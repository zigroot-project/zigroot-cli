# Design Document: Zigroot CLI

## Overview

Zigroot is a Rust CLI application that provides an npm-like experience for building embedded Linux root filesystems. It leverages Zig's cross-compilation capabilities with musl libc to enable building from any host (including macOS) without complex toolchain setup.

The architecture follows a modular design with clear separation between:
- **CLI layer** - Command parsing and user interaction
- **Core layer** - Business logic for package management, building, and image creation
- **Registry layer** - GitHub-based package and board discovery
- **Build layer** - Compilation orchestration using Zig and GCC toolchains

## Coding Practices

### General Principles

1. **No Magic Numbers/Strings** - All constants defined in dedicated modules with descriptive names
   ```rust
   // Bad
   if retries > 3 { ... }
   
   // Good
   const MAX_DOWNLOAD_RETRIES: u32 = 3;
   if retries > MAX_DOWNLOAD_RETRIES { ... }
   ```

2. **Separation of Concerns** - Each module has a single responsibility
   - CLI layer only handles argument parsing and output formatting
   - Core layer contains business logic, no I/O
   - Infrastructure layer handles external interactions (network, filesystem, processes)

3. **Dependency Injection** - Components receive dependencies through constructors, not globals
   ```rust
   // Bad
   impl BuildOrchestrator {
       fn new() -> Self {
           Self { cache: BuildCache::new() }  // Creates its own dependency
       }
   }
   
   // Good
   impl BuildOrchestrator {
       fn new(cache: impl Cache) -> Self {
           Self { cache }  // Receives dependency
       }
   }
   ```

4. **Error Handling** - Use `thiserror` for error types, `anyhow` for application errors
   - All errors include context about what operation failed
   - Errors suggest solutions when possible

5. **Configuration** - All configurable values externalized
   - Default values in `config/defaults.rs`
   - URLs, paths, timeouts as constants, not inline strings

### Module Structure

```
src/
├── main.rs              # Entry point, minimal logic
├── cli/                 # CLI layer
│   ├── mod.rs
│   ├── commands/        # One file per command
│   │   ├── init.rs
│   │   ├── build.rs
│   │   └── ...
│   └── output.rs        # Progress bars, colors, formatting
├── core/                # Business logic (no I/O)
│   ├── mod.rs
│   ├── manifest.rs      # Manifest parsing/validation
│   ├── package.rs       # Package definition handling
│   ├── board.rs         # Board definition handling
│   ├── resolver.rs      # Dependency resolution
│   └── builder.rs       # Build orchestration logic
├── registry/            # Package/board registry client
│   ├── mod.rs
│   ├── client.rs
│   └── cache.rs
├── infra/               # Infrastructure (I/O, external tools)
│   ├── mod.rs
│   ├── download.rs      # HTTP downloads
│   ├── toolchain.rs     # Zig/GCC toolchain management
│   ├── docker.rs        # Docker sandbox
│   └── filesystem.rs    # File operations
├── config/              # Configuration and constants
│   ├── mod.rs
│   ├── defaults.rs      # Default values
│   └── urls.rs          # Registry URLs, toolchain URLs
└── error.rs             # Error types
```

### Extensibility Points

1. **Build Systems** - New build types (e.g., Bazel, Cargo) added by implementing `BuildSystem` trait
2. **Registry Backends** - Support for different registries (GitLab, local) via `Registry` trait
3. **Image Formats** - New output formats via `ImageCreator` trait
4. **Toolchains** - Additional toolchains via `Toolchain` trait

### Testing Strategy

1. **Unit Tests** - Test core logic in isolation with mock dependencies
2. **Integration Tests** - Test CLI commands with real filesystem (in temp dirs)
3. **Property Tests** - Verify invariants with `proptest`

### Code Quality

- `clippy` with pedantic lints enabled
- `rustfmt` for consistent formatting
- Documentation on all public items
- Examples in doc comments for complex functions

### Logging and Diagnostics

Zigroot uses a layered approach to logging and user feedback:

1. **User-Facing Output** (via `indicatif` + custom formatting)
   - Progress bars for downloads and builds
   - Colored status messages (✓ green, ✗ red, ⚠ yellow)
   - Structured output for machine parsing (`--json` flag)

2. **Debug Logging** (via `tracing`)
   - Structured logging with spans for operation context
   - Log levels: ERROR, WARN, INFO, DEBUG, TRACE
   - Default: WARN (errors and warnings only)
   - `--verbose` / `-v`: INFO level
   - `--verbose --verbose` / `-vv`: DEBUG level
   - `ZIGROOT_LOG=trace`: Full trace logging

```rust
use tracing::{info, debug, warn, error, instrument};

#[instrument(skip(self), fields(package = %name))]
async fn build_package(&self, name: &str) -> Result<()> {
    info!("Starting build");
    debug!(srcdir = ?self.srcdir, "Build environment configured");
    
    if let Err(e) = self.run_configure().await {
        error!(error = %e, "Configure failed");
        return Err(e);
    }
    
    info!("Build completed successfully");
    Ok(())
}
```

3. **Build Logs** (persisted to files)
   - Each package build writes to `build/logs/<package>.log`
   - Contains full stdout/stderr from build commands
   - Preserved on failure for debugging
   - Rotated on clean builds

### Error Handling Strategy

1. **Error Types** - Domain-specific errors with `thiserror`

```rust
#[derive(Error, Debug)]
pub enum PackageError {
    #[error("Package '{name}' not found in registry")]
    NotFound { name: String },
    
    #[error("Version constraint '{constraint}' cannot be satisfied for '{package}'")]
    VersionConflict { package: String, constraint: String },
    
    #[error("Checksum mismatch for '{file}': expected {expected}, got {actual}")]
    ChecksumMismatch { file: String, expected: String, actual: String },
}
```

2. **Error Context** - Wrap errors with context using `anyhow`

```rust
use anyhow::{Context, Result};

fn load_manifest(path: &Path) -> Result<Manifest> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read manifest at {}", path.display()))?;
    
    toml::from_str(&content)
        .with_context(|| format!("Failed to parse manifest at {}", path.display()))
}
```

3. **User-Friendly Error Display**

```rust
fn display_error(error: &anyhow::Error) {
    eprintln!("{} {}", "✗".red().bold(), error);
    
    // Show cause chain for debugging
    for cause in error.chain().skip(1) {
        eprintln!("  {} {}", "caused by:".dimmed(), cause);
    }
    
    // Show suggestion if available
    if let Some(suggestion) = get_suggestion(error) {
        eprintln!("\n{} {}", "Suggestion:".yellow(), suggestion);
    }
}
```

4. **Error Recovery Suggestions**

```rust
fn get_suggestion(error: &anyhow::Error) -> Option<String> {
    if let Some(e) = error.downcast_ref::<PackageError>() {
        match e {
            PackageError::NotFound { name } => {
                Some(format!("Run 'zigroot search {}' to find similar packages", name))
            }
            PackageError::ChecksumMismatch { .. } => {
                Some("Try 'zigroot fetch --force' to re-download".to_string())
            }
            _ => None,
        }
    } else if let Some(e) = error.downcast_ref::<std::io::Error>() {
        match e.kind() {
            std::io::ErrorKind::PermissionDenied => {
                Some("Check file permissions or run with appropriate privileges".to_string())
            }
            std::io::ErrorKind::NotFound => {
                Some("Ensure the file exists and the path is correct".to_string())
            }
            _ => None,
        }
    } else {
        None
    }
}
```

5. **Exit Codes**

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Configuration error (invalid manifest, missing required field) |
| 3 | Build error (compilation failed) |
| 4 | Network error (download failed, registry unreachable) |
| 5 | Dependency error (unresolvable, circular) |

## Repository and CI/CD Setup

The zigroot-cli repository (`github.com/zigroot/zigroot-cli`) follows modern Rust project practices:

### Version Embedding

Use `vergen-gitcl` to embed build metadata at compile time:

```rust
// build.rs
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    vergen_gitcl::Emitter::default()
        .add_instructions(
            &vergen_gitcl::BuildBuilder::default()
                .build_timestamp(true)
                .build()?,
        )?
        .add_instructions(
            &vergen_gitcl::CargoBuilder::default()
                .target_triple(true)
                .build()?,
        )?
        .add_instructions(
            &vergen_gitcl::GitclBuilder::default()
                .sha(true)
                .commit_timestamp(true)
                .dirty(true)
                .build()?,
        )?
        .add_instructions(&vergen_gitcl::RustcBuilder::default().semver(true).build()?)?
        .emit()?;
    Ok(())
}
```

This enables `zigroot --version` to show:
```
zigroot 0.1.0 (abc1234 2025-01-11, rustc 1.83.0, x86_64-apple-darwin)
```

### Conventional Commits

All commits must follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add support for SquashFS images
fix: correct checksum verification for large files
docs: update README with installation instructions
perf: optimize dependency resolution algorithm
refactor: extract toolchain logic into separate module
test: add property tests for manifest serialization
chore: update dependencies
```

**Setup files:**

```javascript
// commitlint.config.js
module.exports = { extends: ['@commitlint/config-conventional'] };
```

```json
// package.json
{
  "name": "zigroot-cli",
  "private": true,
  "devDependencies": {
    "@commitlint/cli": "^19.0.0",
    "@commitlint/config-conventional": "^19.0.0",
    "husky": "^9.0.0"
  },
  "scripts": {
    "prepare": "husky"
  }
}
```

### Changelog Generation

Use `git-cliff` for automatic changelog generation from conventional commits:

```toml
# cliff.toml
[changelog]
header = """
# Changelog

All notable changes to this project will be documented in this file.

"""
body = """
{% if version %}\
    ## [{{ version | trim_start_matches(pat="v") }}] - {{ timestamp | date(format="%Y-%m-%d") }}
{% else %}\
    ## [Unreleased]
{% endif %}\
{% for group, commits in commits | group_by(attribute="group") %}
    ### {{ group | upper_first }}
    {% for commit in commits %}
        - {{ commit.message | upper_first }}\
    {% endfor %}
{% endfor %}\n
"""
trim = true

[git]
conventional_commits = true
filter_unconventional = true
commit_parsers = [
    { message = "^feat", group = "Added" },
    { message = "^fix", group = "Fixed" },
    { message = "^docs", group = "Documentation" },
    { message = "^perf", group = "Performance" },
    { message = "^refactor", group = "Refactored" },
    { message = "^test", group = "Testing" },
    { message = "^chore\\(release\\)", skip = true },
    { message = "^chore", group = "Miscellaneous" },
    { body = ".*security", group = "Security" },
]
tag_pattern = "v[0-9]*"
sort_commits = "oldest"
```

### Release Automation

Use `release-plz` for automated releases:

```toml
# release-plz.toml
[workspace]
changelog_config = "cliff.toml"

[[package]]
name = "zigroot"
changelog_include = ["feat", "fix", "docs", "perf", "refactor"]
git_release_enable = true
publish = true
```

### GitHub Actions CI

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  test:
    name: Test
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest]
        rust: [stable, beta]
    steps:
      - uses: actions/checkout@v6
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: ${{ matrix.rust }}
      - uses: Swatinem/rust-cache@v2
      - run: cargo test --all-features

  clippy:
    name: Clippy
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo clippy --all-targets --all-features -- -D warnings

  fmt:
    name: Format
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt
      - run: cargo fmt --all -- --check

  deny:
    name: Cargo Deny
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
      - uses: EmbarkStudios/cargo-deny-action@v2

  security:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
      - uses: rustsec/audit-check@v2
        with:
          token: ${{ secrets.GITHUB_TOKEN }}

  commitlint:
    name: Lint Commits
    runs-on: ubuntu-latest
    if: github.event_name == 'pull_request'
    steps:
      - uses: actions/checkout@v6
        with:
          fetch-depth: 0
      - uses: actions/setup-node@v4
        with:
          node-version: '20'
      - run: npm install
      - run: npx commitlint --from ${{ github.event.pull_request.base.sha }} --to ${{ github.event.pull_request.head.sha }} --verbose
```

### Release Workflow

```yaml
# .github/workflows/release.yml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    name: Build ${{ matrix.target }}
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - os: ubuntu-latest
            target: x86_64-unknown-linux-musl
          - os: ubuntu-latest
            target: aarch64-unknown-linux-gnu
          - os: ubuntu-latest
            target: aarch64-unknown-linux-musl
          - os: macos-latest
            target: x86_64-apple-darwin
          - os: macos-latest
            target: aarch64-apple-darwin
    steps:
      - uses: actions/checkout@v6
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - uses: Swatinem/rust-cache@v2
      - name: Build
        run: cargo build --release --target ${{ matrix.target }}
      - name: Create archive
        run: |
          VERSION=${GITHUB_REF#refs/tags/v}
          cp target/${{ matrix.target }}/release/zigroot .
          tar czf "zigroot-${VERSION}-${{ matrix.target }}.tar.gz" zigroot README.md LICENSE
      - uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.target }}
          path: zigroot-*.tar.gz

  release:
    needs: build
    runs-on: ubuntu-latest
    outputs:
      version: ${{ steps.version.outputs.version }}
      is_prerelease: ${{ steps.version.outputs.is_prerelease }}
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v6
      - name: Determine version
        id: version
        run: |
          VERSION=${GITHUB_REF#refs/tags/v}
          if [[ "$VERSION" =~ (alpha|beta|rc|-) ]]; then
            IS_PRERELEASE=true
          else
            IS_PRERELEASE=false
          fi
          echo "version=${VERSION}" >> "$GITHUB_OUTPUT"
          echo "is_prerelease=${IS_PRERELEASE}" >> "$GITHUB_OUTPUT"
      - uses: actions/download-artifact@v4
        with:
          path: artifacts
      - name: Generate changelog
        run: |
          cargo install git-cliff
          git cliff --tag v${{ steps.version.outputs.version }} --output RELEASE_NOTES.md
      - uses: softprops/action-gh-release@v2
        with:
          files: artifacts/**/*.tar.gz
          prerelease: ${{ steps.version.outputs.is_prerelease == 'true' }}
          body_path: RELEASE_NOTES.md

  publish-crates:
    name: Publish to crates.io
    needs: release
    if: needs.release.outputs.is_prerelease == 'false'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo publish --token ${{ secrets.CARGO_REGISTRY_TOKEN }}

  publish-homebrew:
    name: Update Homebrew tap
    needs: release
    if: needs.release.outputs.is_prerelease == 'false'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/download-artifact@v4
        with:
          path: artifacts
      - name: Calculate checksums
        id: checksums
        run: |
          VERSION=${{ needs.release.outputs.version }}
          X64_SHA=$(sha256sum artifacts/x86_64-apple-darwin/zigroot-${VERSION}-x86_64-apple-darwin.tar.gz | cut -d' ' -f1)
          ARM64_SHA=$(sha256sum artifacts/aarch64-apple-darwin/zigroot-${VERSION}-aarch64-apple-darwin.tar.gz | cut -d' ' -f1)
          echo "x64_sha=${X64_SHA}" >> "$GITHUB_OUTPUT"
          echo "arm64_sha=${ARM64_SHA}" >> "$GITHUB_OUTPUT"
      - name: Update Homebrew tap
        env:
          TAP_TOKEN: ${{ secrets.HOMEBREW_TAP_TOKEN }}
        run: |
          VERSION=${{ needs.release.outputs.version }}
          git clone "https://x-access-token:${TAP_TOKEN}@github.com/zigroot/homebrew-tap.git"
          cd homebrew-tap
          mkdir -p Formula
          cat > Formula/zigroot.rb << EOF
          class Zigroot < Formula
            desc "Modern embedded Linux rootfs builder - npm for embedded systems"
            homepage "https://github.com/zigroot/zigroot-cli"
            version "${VERSION}"
            license "MIT"

            on_macos do
              if Hardware::CPU.intel?
                url "https://github.com/zigroot/zigroot-cli/releases/download/v${VERSION}/zigroot-${VERSION}-x86_64-apple-darwin.tar.gz"
                sha256 "${{ steps.checksums.outputs.x64_sha }}"
              else
                url "https://github.com/zigroot/zigroot-cli/releases/download/v${VERSION}/zigroot-${VERSION}-aarch64-apple-darwin.tar.gz"
                sha256 "${{ steps.checksums.outputs.arm64_sha }}"
              end
            end

            def install
              bin.install "zigroot"
            end

            test do
              assert_match version.to_s, shell_output("#{bin}/zigroot --version")
            end
          end
          EOF
          git config user.name "github-actions[bot]"
          git config user.email "github-actions[bot]@users.noreply.github.com"
          git add Formula/zigroot.rb
          git commit -m "Update zigroot to ${VERSION}"
          git push

  publish-aur:
    name: Publish to AUR
    needs: release
    if: needs.release.outputs.is_prerelease == 'false'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
      - name: Update PKGBUILD version
        run: |
          sed -i "s/^pkgver=.*/pkgver=${{ needs.release.outputs.version }}/" aur/PKGBUILD
          sed -i "s/^pkgrel=.*/pkgrel=1/" aur/PKGBUILD
      - name: Publish to AUR
        uses: KSXGitHub/github-actions-deploy-aur@v4.1.1
        with:
          pkgname: zigroot
          pkgbuild: ./aur/PKGBUILD
          commit_username: zigroot
          commit_email: aur@zigroot.dev
          ssh_private_key: ${{ secrets.AUR_SSH_PRIVATE_KEY }}
          commit_message: "Update to ${{ needs.release.outputs.version }}"
```

### AUR Package

```bash
# aur/PKGBUILD
# Maintainer: zigroot <aur@zigroot.dev>
pkgname=zigroot
pkgver=0.1.0
pkgrel=1
pkgdesc="Modern embedded Linux rootfs builder - npm for embedded systems"
arch=('x86_64' 'aarch64')
url="https://github.com/zigroot/zigroot-cli"
license=('MIT')
depends=()
makedepends=('cargo')
source=("$pkgname-$pkgver.tar.gz::https://github.com/zigroot/$pkgname-cli/archive/refs/tags/v$pkgver.tar.gz")
sha256sums=('SKIP')

prepare() {
    cd "$pkgname-cli-$pkgver"
    export RUSTUP_TOOLCHAIN=stable
    cargo fetch --locked --target "$(rustc -vV | sed -n 's/host: //p')"
}

build() {
    cd "$pkgname-cli-$pkgver"
    export RUSTUP_TOOLCHAIN=stable
    export CARGO_TARGET_DIR=target
    cargo build --frozen --release
}

check() {
    cd "$pkgname-cli-$pkgver"
    export RUSTUP_TOOLCHAIN=stable
    cargo test --frozen
}

package() {
    cd "$pkgname-cli-$pkgver"
    install -Dm755 "target/release/$pkgname" "$pkgdir/usr/bin/$pkgname"
    install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
    install -Dm644 README.md "$pkgdir/usr/share/doc/$pkgname/README.md"
}
```

### Distribution Summary

| Platform | Method | Repository |
|----------|--------|------------|
| crates.io | `cargo install zigroot` | crates.io/crates/zigroot |
| Homebrew | `brew install zigroot/tap/zigroot` | github.com/zigroot/homebrew-tap |
| AUR | `yay -S zigroot` | aur.archlinux.org/packages/zigroot |
| GitHub | Download binary | github.com/zigroot/zigroot-cli/releases |

### GitHub Organization Setup

The `zigroot` GitHub organization requires the following repositories:

| Repository | Purpose |
|------------|---------|
| `zigroot/zigroot-cli` | Main CLI tool (this project) |
| `zigroot/zigroot-packages` | Community package definitions |
| `zigroot/zigroot-boards` | Board definitions for various hardware |
| `zigroot/homebrew-tap` | Homebrew formula for macOS installation |

**Required Secrets** (in `zigroot/zigroot-cli` repository settings):

| Secret | Purpose |
|--------|---------|
| `CARGO_REGISTRY_TOKEN` | crates.io API token for publishing |
| `HOMEBREW_TAP_TOKEN` | GitHub PAT with write access to `zigroot/homebrew-tap` |
| `AUR_SSH_PRIVATE_KEY` | SSH key registered with AUR for publishing |

**Organization Setup Steps:**

1. Create GitHub organization `zigroot`
2. Create repositories:
   - `zigroot-cli` (main CLI)
   - `zigroot-packages` (package registry)
   - `zigroot-boards` (board registry)
   - `homebrew-tap` (Homebrew formulas)
3. Configure secrets in `zigroot-cli`:
   - Generate crates.io token at https://crates.io/settings/tokens
   - Create GitHub PAT with `repo` scope for homebrew-tap
   - Generate SSH key and register at https://aur.archlinux.org
4. Enable GitHub Actions in all repositories
5. Set up branch protection rules on `main`

**Registry Repository Structure:**

```
zigroot-packages/
├── index.json           # Auto-generated: all packages with all versions
├── packages/
│   ├── busybox/
│   │   ├── metadata.toml   # Package metadata (name, description, license, build)
│   │   ├── 1.36.1.toml     # Version-specific (url, sha256, deps for this version)
│   │   └── 1.36.0.toml
│   ├── dropbear/
│   │   ├── metadata.toml
│   │   └── 2024.85.toml
│   └── ...
├── .github/
│   └── workflows/
│       └── update-index.yml  # Auto-generates index.json on push
└── README.md

zigroot-boards/
├── index.json           # Auto-generated: all boards
├── boards/
│   ├── luckfox-pico/
│   │   └── board.toml      # Board definition (single file, no versioning)
│   ├── raspberry-pi-4/
│   │   └── board.toml
│   └── ...
├── .github/
│   └── workflows/
│       └── update-index.yml
└── README.md
```

**Package File Structure:**

Registry packages are split into two file types to minimize duplication:

| File | Purpose | Contents |
|------|---------|----------|
| `metadata.toml` | Shared across all versions | name, description, license, homepage, keywords, build config, options |
| `<version>.toml` | Version-specific | version, released date, source URL, sha256, version-specific deps |

This split means:
- Adding a new version only requires creating one small file
- Build instructions and options are defined once
- Version-specific dependencies can override metadata defaults

```toml
# packages/busybox/metadata.toml - Shared across all versions
[package]
name = "busybox"
description = "Swiss army knife of embedded Linux"
license = "GPL-2.0"
homepage = "https://busybox.net"
keywords = ["shell", "coreutils", "init"]
# zigroot_version = ">=0.2.0"  # Optional: minimum zigroot version required

[build]
type = "make"
make_args = ["CROSS_COMPILE=${TARGET}-"]

[options.shell]
type = "choice"
default = "ash"
choices = ["ash", "hush", "none"]
description = "Default shell"
```

```toml
# packages/busybox/1.36.1.toml - Version-specific
[release]
version = "1.36.1"
released = "2024-01-15"

[source]
url = "https://busybox.net/downloads/busybox-1.36.1.tar.bz2"
sha256 = "b8cc24c9574d809e7279c3be349795c5d5ceb6fdf19ca709f80cde50e47de314"

# Version-specific dependencies (if any differ from metadata.toml)
# depends = ["zlib>=1.2.11"]
```

**Package Maintainer Workflow:**

1. **Create new package:**
   ```bash
   zigroot package new mypackage
   # Creates packages/mypackage/metadata.toml and packages/mypackage/1.0.0.toml
   ```

2. **Edit the generated files:**
   ```bash
   $EDITOR packages/mypackage/metadata.toml
   $EDITOR packages/mypackage/1.0.0.toml
   ```

3. **Validate the package:**
   ```bash
   zigroot verify packages/mypackage
   # Checks TOML syntax, required fields, etc.
   
   zigroot verify --fetch packages/mypackage
   # Also downloads source and verifies SHA256
   ```

4. **Test build:**
   ```bash
   zigroot package test packages/mypackage
   # Attempts to build the package
   ```

5. **Publish to registry:**
   ```bash
   export GITHUB_TOKEN=ghp_xxx  # Or use `gh auth login`
   zigroot publish packages/mypackage
   # Creates PR to zigroot/zigroot-packages
   ```

6. **Add new version:**
   ```bash
   zigroot package bump packages/mypackage 1.1.0
   # Creates packages/mypackage/1.1.0.toml from latest version
   # Edit the new file with updated url/sha256
   zigroot publish packages/mypackage
   ```

**Board Maintainer Workflow:**

1. **Create new board:**
   ```bash
   zigroot board new myboard
   # Creates boards/myboard/board.toml
   ```

2. **Edit and validate:**
   ```bash
   $EDITOR boards/myboard/board.toml
   zigroot verify boards/myboard
   ```

3. **Publish:**
   ```bash
   zigroot publish boards/myboard
   # Creates PR to zigroot/zigroot-boards
   ```

**Publishing Implementation:**

```rust
impl PackagePublisher {
    pub async fn publish(&self, path: &Path) -> Result<PullRequestUrl> {
        // 1. Validate package
        self.validate(path)?;
        
        // 2. Check for conflicts
        let metadata = self.load_metadata(path)?;
        if self.package_exists(&metadata.name).await? {
            // Existing package - only allow adding new versions
            self.validate_new_version_only(path).await?;
        }
        
        // 3. Fork repo (if needed) and create branch
        let branch = format!("add-{}-{}", metadata.name, Utc::now().timestamp());
        let fork = self.ensure_fork().await?;
        
        // 4. Copy files to fork
        self.copy_package_to_fork(path, &fork, &branch).await?;
        
        // 5. Create PR
        let pr = self.create_pull_request(&fork, &branch, &metadata).await?;
        
        Ok(pr.url)
    }
}
```

**Index Generation (GitHub Action):**

```yaml
# .github/workflows/update-index.yml
name: Update Index

on:
  push:
    branches: [main]
    paths: ['packages/**']

jobs:
  update-index:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
      
      - name: Generate index
        run: |
          python3 << 'EOF'
          import json, tomllib, os
          from pathlib import Path
          
          packages = []
          for pkg_dir in Path("packages").iterdir():
              if not pkg_dir.is_dir():
                  continue
              
              metadata_file = pkg_dir / "metadata.toml"
              if not metadata_file.exists():
                  continue
              
              with open(metadata_file, "rb") as f:
                  metadata = tomllib.load(f)
              
              versions = []
              for version_file in pkg_dir.glob("*.toml"):
                  if version_file.name == "metadata.toml":
                      continue
                  with open(version_file, "rb") as f:
                      ver_data = tomllib.load(f)
                  versions.append({
                      "version": ver_data["version"]["version"],
                      "released": ver_data["version"].get("released"),
                      "sha256": ver_data["source"]["sha256"]
                  })
              
              versions.sort(key=lambda v: v["version"], reverse=True)
              
              packages.append({
                  "name": metadata["package"]["name"],
                  "description": metadata["package"]["description"],
                  "license": metadata["package"].get("license"),
                  "keywords": metadata["package"].get("keywords", []),
                  "versions": versions,
                  "latest": versions[0]["version"] if versions else None
              })
          
          index = {
              "version": 1,
              "updated": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
              "packages": sorted(packages, key=lambda p: p["name"])
          }
          
          with open("index.json", "w") as f:
              json.dump(index, f, indent=2)
          EOF
      
      - name: Commit index
        run: |
          git config user.name "github-actions[bot]"
          git config user.email "github-actions[bot]@users.noreply.github.com"
          git add index.json
          git diff --staged --quiet || git commit -m "chore: update index.json"
          git push
```

### Cargo Deny Configuration

```toml
# deny.toml
[advisories]
vulnerability = "deny"
unmaintained = "warn"

[licenses]
allow = ["MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Zlib"]
copyleft = "warn"

[bans]
multiple-versions = "warn"
wildcards = "deny"
```

## Architecture

### Toolchain Philosophy

Zigroot uses Zig's built-in cross-compilation for everything by default:

1. **Zig (default)** - Used for ALL packages unless explicitly overridden. Zig's built-in cross-compilation with musl libc provides:
   - Zero-setup cross-compilation to any target
   - Static linking by default
   - Works from macOS, Linux, or Windows
   - No external toolchain downloads needed

2. **GCC (package-declared)** - Some packages (Linux kernel, U-Boot) cannot be built with Zig. These packages declare their toolchain needs in `package.toml`:
   - **Simple approach**: Specify `type = "gcc"` and `target` - Zigroot auto-resolves bootlin.com URLs
   - **Explicit approach**: Specify `type = "gcc-explicit"` with custom URLs per host platform
   - Toolchains are downloaded and cached automatically

This keeps toolchain complexity where it belongs - in the packages that need it - while making the common case (bootlin toolchains) trivially easy.

```
┌─────────────────────────────────────────────────────────────────┐
│                         CLI Layer                               │
│ ┌───────┐┌───────┐┌───────┐┌───────┐┌───────┐┌───────┐┌───────┐│
│ │ init  ││  add  ││ build ││ fetch ││search ││ tree  ││publish││
│ └───┬───┘└───┬───┘└───┬───┘└───┬───┘└───┬───┘└───┬───┘└───┬───┘│
│ ┌───────┐┌───────┐┌───────┐┌───────┐┌───────┐┌───────┐┌───────┐│
│ │verify ││ flash ││config ││doctor ││  sdk  ││license││ cache ││
│ └───┬───┘└───┬───┘└───┬───┘└───┬───┘└───┬───┘└───┬───┘└───┬───┘│
└─────┼────────┼────────┼────────┼────────┼────────┼────────┼────┘
      │        │        │        │        │        │        │
┌─────┴────────┴────────┴────────┴────────┴────────┴────────┴────┐
│                         Core Layer                              │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐             │
│  │   Manifest   │ │   Package    │ │    Board     │             │
│  │   Manager    │ │   Resolver   │ │   Manager    │             │
│  └──────────────┘ └──────────────┘ └──────────────┘             │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐             │
│  │  Dependency  │ │    Build     │ │    Image     │             │
│  │   Resolver   │ │  Orchestrator│ │   Creator    │             │
│  └──────────────┘ └──────────────┘ └──────────────┘             │
└─────────────────────────────────────────────────────────────────┘
        │                    │               │
┌───────┴────────────────────┴───────────────┴────────────────────┐
│                       Infrastructure Layer                      │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐             │
│  │   Registry   │ │   Download   │ │    Cache     │             │
│  │    Client    │ │   Manager    │ │   Manager    │             │
│  └──────────────┘ └──────────────┘ └──────────────┘             │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐             │
│  │  Zig/GCC     │ │   Docker     │ │   Progress   │             │
│  │  Toolchain   │ │   Sandbox    │ │   Reporter   │             │
│  └──────────────┘ └──────────────┘ └──────────────┘             │
└─────────────────────────────────────────────────────────────────┘
```

## Components and Interfaces

### CLI Commands

```rust
// Global flags available on all commands
#[derive(Parser)]
#[command(name = "zigroot", version, about)]
pub struct Cli {
    /// Increase verbosity (-v info, -vv debug)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,
    
    /// Suppress all output except errors
    #[arg(short, long, global = true)]
    pub quiet: bool,
    
    /// Output in JSON format (for scripting)
    #[arg(long, global = true)]
    pub json: bool,
    
    #[command(subcommand)]
    pub command: Command,
}

// Main command structure using clap (Cargo-aligned)
#[derive(Subcommand)]
pub enum Command {
    // Project management
    Init(InitArgs),
    
    // Package management (top-level like Cargo)
    Add(AddArgs),
    Remove(RemoveArgs),
    Update(UpdateArgs),
    
    // Build commands
    Fetch(FetchArgs),         // Download sources (like cargo fetch)
    Build(BuildArgs),
    Check,                    // Validate without building (like cargo check)
    Clean,                    // Remove build artifacts (like cargo clean)
    
    // Publishing
    Publish(PublishArgs),
    Verify(VerifyArgs),       // Validate package/board (like cargo verify-project)
    
    // Discovery
    Search(SearchArgs),
    Tree(TreeArgs),           // Dependency tree (like cargo tree)
    
    // Subcommands for less common operations
    Package(PackageCommand),
    Board(BoardCommand),
    Flash(FlashArgs),
    Config(ConfigArgs),
    Doctor,
    Sdk(SdkArgs),
    License(LicenseArgs),
    Cache(CacheCommand),
    Kernel(KernelCommand),
    External(ExternalCommand),
}

// ============================================
// Project Initialization
// ============================================

#[derive(Args)]
pub struct InitArgs {
    /// Board to configure for the project
    #[arg(long)]
    pub board: Option<String>,
    
    /// Force initialization in non-empty directory
    #[arg(long)]
    pub force: bool,
}

// ============================================
// Package Management (Top-Level)
// ============================================

#[derive(Args)]
pub struct AddArgs {
    /// Package name, optionally with version (e.g., busybox@1.36.1)
    pub package: String,
    
    /// Add package from a git repository (e.g., https://github.com/user/repo)
    #[arg(long)]
    pub git: Option<String>,
    
    /// Git tag to use (with --git)
    #[arg(long)]
    pub tag: Option<String>,
    
    /// Git branch to use (with --git)
    #[arg(long)]
    pub branch: Option<String>,
    
    /// Git commit to use (with --git)
    #[arg(long)]
    pub rev: Option<String>,
    
    /// Add package from a custom registry URL
    #[arg(long)]
    pub registry: Option<String>,
    
    /// Add package from a local path
    #[arg(long)]
    pub path: Option<PathBuf>,
}

#[derive(Args)]
pub struct RemoveArgs {
    /// Package name to remove
    pub package: String,
}

#[derive(Args)]
pub struct UpdateArgs {
    /// Package to update (all if not specified)
    pub package: Option<String>,
    
    /// Check for zigroot updates (instead of package updates)
    #[arg(long)]
    pub self_update: bool,
    
    /// Install the latest zigroot version (use with --self)
    #[arg(long)]
    pub install: bool,
}

// ============================================
// Build Commands
// ============================================
#[derive(Args)]
pub struct FetchArgs {
    /// Number of parallel downloads
    #[arg(long)]
    pub parallel: Option<usize>,
    
    /// Force re-download even if files exist
    #[arg(long)]
    pub force: bool,
}

#[derive(Args)]
pub struct BuildArgs {
    /// Build only the specified package
    #[arg(long)]
    pub package: Option<String>,
    
    /// Number of parallel jobs
    #[arg(short, long)]
    pub jobs: Option<usize>,
    
    /// Enable compression (overrides config)
    #[arg(long)]
    pub compress: bool,
    
    /// Disable compression (overrides config)
    #[arg(long)]
    pub no_compress: bool,
    
    /// Use Docker/Podman sandbox for builds
    #[arg(long)]
    pub sandbox: bool,
    
    /// Disable sandbox (overrides config)
    #[arg(long)]
    pub no_sandbox: bool,
    
    /// Build only the kernel and modules
    #[arg(long)]
    pub kernel_only: bool,
    
    /// Fail if any package differs from lock file
    #[arg(long)]
    pub locked: bool,
}

// ============================================
// Publishing & Verification
// ============================================

#[derive(Args)]
pub struct PublishArgs {
    /// Path to package or board to publish
    pub path: PathBuf,
}

#[derive(Args)]
pub struct VerifyArgs {
    /// Path to package or board to verify
    pub path: PathBuf,
    
    /// Also download and verify source checksums
    #[arg(long)]
    pub fetch: bool,
}

// ============================================
// Discovery Commands
// ============================================

#[derive(Args)]
pub struct SearchArgs {
    /// Search query
    pub query: String,
    
    /// Search only packages
    #[arg(long)]
    pub packages: bool,
    
    /// Search only boards
    #[arg(long)]
    pub boards: bool,
    
    /// Force refresh of indexes before searching
    #[arg(long)]
    pub refresh: bool,
}

#[derive(Args)]
pub struct TreeArgs {
    /// Show dependencies for specific package
    pub package: Option<String>,
    
    /// Output as DOT graph format
    #[arg(long)]
    pub graph: bool,
}

// ============================================
// Package Subcommand
// ============================================

#[derive(Subcommand)]
pub enum PackageCommand {
    /// Create new package template
    New { name: String },
    /// Show package information
    Info { name: String },
    /// List installed packages
    List,
    /// Test build a local package
    Test { path: PathBuf },
    /// Create new version from latest
    Bump { path: PathBuf, version: String },
}

// ============================================
// Board Subcommand
// ============================================

#[derive(Subcommand)]
pub enum BoardCommand {
    /// List available boards
    List,
    /// Set project board
    Set { name: String },
    /// Show board information
    Info { name: String },
    /// Create new board template
    New { name: String },
}

// ============================================
// Flash Command
// ============================================

#[derive(Args)]
pub struct FlashArgs {
    /// Flash method to use (lists available if not specified)
    pub method: Option<String>,
    
    /// Device path to flash to
    #[arg(long)]
    pub device: Option<PathBuf>,
    
    /// Skip confirmation prompt
    #[arg(long, short)]
    pub yes: bool,
    
    /// List available flash methods
    #[arg(long)]
    pub list: bool,
}

// ============================================
// Config Command (TUI)
// ============================================

#[derive(Args)]
pub struct ConfigArgs {
    /// Show only board selection
    #[arg(long)]
    pub board: bool,
    
    /// Show only package selection
    #[arg(long)]
    pub packages: bool,
}

/*
TUI Interface Design (using ratatui):

┌─────────────────────────────────────────────────────────────────┐
│ zigroot config                                          [?] Help│
├─────────────────────────────────────────────────────────────────┤
│ ▼ Board                                                         │
│   ● luckfox-pico         Luckfox Pico (RV1103, Cortex-A7)      │
│   ○ raspberry-pi-4       Raspberry Pi 4 Model B                 │
│   ○ beaglebone-black     BeagleBone Black (AM335x)              │
│                                                                 │
│ ▼ Packages                                          [Space] Toggle│
│   [✓] busybox 1.36.1     Swiss army knife of embedded Linux    │
│   [✓] dropbear 2024.85   Lightweight SSH server                │
│   [ ] nginx 1.24.0       HTTP server                           │
│   [ ] sqlite 3.44.0      Embedded database                     │
│                                                                 │
│ ▶ Build Options                                                 │
│ ▶ External Artifacts                                            │
├─────────────────────────────────────────────────────────────────┤
│ [↑↓] Navigate  [Space] Select  [Enter] Expand  [/] Search      │
│ [s] Save       [q] Quit        [d] Show diff   [?] Help         │
└─────────────────────────────────────────────────────────────────┘

When a package with options is selected, pressing Enter shows:

┌─────────────────────────────────────────────────────────────────┐
│ busybox 1.36.1 - Options                                        │
├─────────────────────────────────────────────────────────────────┤
│ shell          [ash ▼]    Default shell (ash, hush, none)       │
│ syslogd        [✓]        Enable syslogd                        │
│ httpd          [ ]        Enable httpd web server               │
│ ftpd           [ ]        Enable FTP server                     │
├─────────────────────────────────────────────────────────────────┤
│ [Enter] Change  [Esc] Back  [d] Reset to default                │
└─────────────────────────────────────────────────────────────────┘

Before saving, shows diff:

┌─────────────────────────────────────────────────────────────────┐
│ Changes to zigroot.toml                                         │
├─────────────────────────────────────────────────────────────────┤
│ + [packages]                                                    │
│ + busybox = "1.36.1"                                            │
│ + dropbear = "2024.85"                                          │
│ +                                                               │
│ + [packages.busybox.options]                                    │
│ + shell = "ash"                                                 │
├─────────────────────────────────────────────────────────────────┤
│ [y] Save and exit  [n] Continue editing  [q] Discard changes    │
└─────────────────────────────────────────────────────────────────┘
*/

// ============================================
// SDK Command
// ============================================

#[derive(Args)]
pub struct SdkArgs {
    /// Output path for SDK tarball
    #[arg(long)]
    pub output: Option<PathBuf>,
}

// ============================================
// License Command
// ============================================

#[derive(Args)]
pub struct LicenseArgs {
    /// Export license report to file
    #[arg(long)]
    pub export: Option<PathBuf>,
    
    /// Generate SPDX-compatible SBOM
    #[arg(long)]
    pub sbom: bool,
}

// ============================================
// Cache Subcommand
// ============================================

#[derive(Subcommand)]
pub enum CacheCommand {
    /// Export cache to tarball
    Export { path: PathBuf },
    /// Import cache from tarball
    Import { path: PathBuf },
    /// Show cache statistics
    Info,
    /// Clear local cache
    Clean,
}

// ============================================
// Kernel Subcommand
// ============================================

#[derive(Subcommand)]
pub enum KernelCommand {
    /// Launch kernel menuconfig interface
    Menuconfig,
    /// Show current kernel configuration
    Config,
}

// ============================================
// External Artifacts Subcommand
// ============================================

#[derive(Subcommand)]
pub enum ExternalCommand {
    /// List configured external artifacts
    List,
    /// Add external artifact to manifest
    Add {
        /// Artifact name
        name: String,
        /// Artifact type (bootloader, kernel, partition_table, dtb, firmware, other)
        #[arg(long, short = 't')]
        artifact_type: String,
        /// Remote URL for artifact
        #[arg(long)]
        url: Option<String>,
        /// Local path for artifact
        #[arg(long)]
        path: Option<PathBuf>,
        /// SHA256 checksum (required for URL)
        #[arg(long)]
        sha256: Option<String>,
    },
    /// Remove external artifact from manifest
    Remove { name: String },
}
```

### CLI Command Summary

| Command | Description | Cargo Equivalent |
|---------|-------------|------------------|
| `zigroot init` | Initialize new project | `cargo init` |
| `zigroot add <pkg>` | Add package from registry | `cargo add` |
| `zigroot add <pkg> --git <url>` | Add package from git repo | `cargo add --git` |
| `zigroot add <pkg> --path <dir>` | Add local package | `cargo add --path` |
| `zigroot add <pkg> --registry <url>` | Add from custom registry | `cargo add --registry` |
| `zigroot remove <pkg>` | Remove package | `cargo remove` |
| `zigroot update [pkg]` | Update packages | `cargo update` |
| `zigroot update --self` | Check for zigroot updates | `rustup update` |
| `zigroot fetch` | Download all sources | `cargo fetch` |
| `zigroot build` | Build rootfs image | `cargo build` |
| `zigroot check` | Validate without building | `cargo check` |
| `zigroot clean` | Remove build artifacts | `cargo clean` |
| `zigroot tree` | Show dependency tree | `cargo tree` |
| `zigroot search <q>` | Search packages/boards | `cargo search` |
| `zigroot verify <path>` | Validate package/board | `cargo verify-project` |
| `zigroot publish <path>` | Publish to registry | `cargo publish` |
| `zigroot package *` | Package management | - |
| `zigroot board *` | Board management | - |
| `zigroot flash` | Flash to device | - |
| `zigroot config` | Interactive TUI config | - |
| `zigroot doctor` | Check system deps | - |
| `zigroot sdk` | Generate SDK tarball | - |
| `zigroot license` | License compliance | `cargo license` |
| `zigroot cache *` | Cache management | - |
| `zigroot kernel *` | Kernel configuration | - |
| `zigroot external *` | External artifacts | - |
```

### Update Checker

Zigroot checks for updates and enforces minimum version requirements:

```rust
pub struct UpdateChecker {
    http_client: reqwest::Client,
    cache_dir: PathBuf,
    current_version: Version,
}

impl UpdateChecker {
    /// Check GitHub releases for newer versions
    pub async fn check_for_updates(&self) -> Result<Option<UpdateInfo>> {
        // Check cache first (max once per day)
        if let Some(cached) = self.load_cached_check()? {
            if cached.checked_at > Utc::now() - Duration::days(1) {
                return Ok(cached.update_info);
            }
        }
        
        // Query GitHub releases API
        let releases = self.fetch_releases().await?;
        let latest = releases.iter()
            .filter(|r| !r.prerelease)
            .max_by_key(|r| &r.version)?;
        
        let update_info = if latest.version > self.current_version {
            Some(UpdateInfo {
                current: self.current_version.clone(),
                latest: latest.version.clone(),
                release_url: latest.html_url.clone(),
                release_notes: latest.body.clone(),
            })
        } else {
            None
        };
        
        // Cache result
        self.save_cached_check(&update_info)?;
        
        Ok(update_info)
    }
    
    /// Verify package/board zigroot_version constraint
    pub fn check_version_constraint(&self, constraint: &str) -> Result<()> {
        let req = VersionReq::parse(constraint)
            .with_context(|| format!("Invalid version constraint: {}", constraint))?;
        
        if !req.matches(&self.current_version) {
            return Err(ZigrootError::VersionMismatch {
                required: constraint.to_string(),
                current: self.current_version.to_string(),
                suggestion: format!(
                    "Update zigroot with: zigroot update --self --install\n\
                     Or download from: https://github.com/zigroot/zigroot-cli/releases"
                ),
            });
        }
        
        Ok(())
    }
    
    /// Detect installation method and perform self-update
    pub async fn self_update(&self) -> Result<()> {
        let method = self.detect_install_method()?;
        
        match method {
            InstallMethod::Cargo => {
                println!("Updating via cargo...");
                Command::new("cargo")
                    .args(["install", "zigroot", "--force"])
                    .status()?;
            }
            InstallMethod::Homebrew => {
                println!("Updating via homebrew...");
                Command::new("brew")
                    .args(["upgrade", "zigroot/tap/zigroot"])
                    .status()?;
            }
            InstallMethod::Aur => {
                println!("Updating via AUR...");
                println!("Run: yay -Syu zigroot");
            }
            InstallMethod::Binary => {
                println!("Manual installation detected.");
                println!("Download latest from: https://github.com/zigroot/zigroot-cli/releases");
            }
        }
        
        Ok(())
    }
    
    fn detect_install_method(&self) -> Result<InstallMethod> {
        // Check if installed via cargo
        if which::which("cargo").is_ok() {
            let output = Command::new("cargo")
                .args(["install", "--list"])
                .output()?;
            if String::from_utf8_lossy(&output.stdout).contains("zigroot") {
                return Ok(InstallMethod::Cargo);
            }
        }
        
        // Check if installed via homebrew
        if which::which("brew").is_ok() {
            let output = Command::new("brew")
                .args(["list", "--formula"])
                .output()?;
            if String::from_utf8_lossy(&output.stdout).contains("zigroot") {
                return Ok(InstallMethod::Homebrew);
            }
        }
        
        // Check if installed via AUR (pacman)
        if which::which("pacman").is_ok() {
            let output = Command::new("pacman")
                .args(["-Q", "zigroot"])
                .output()?;
            if output.status.success() {
                return Ok(InstallMethod::Aur);
            }
        }
        
        Ok(InstallMethod::Binary)
    }
}

pub struct UpdateInfo {
    pub current: Version,
    pub latest: Version,
    pub release_url: String,
    pub release_notes: Option<String>,
}

pub enum InstallMethod {
    Cargo,
    Homebrew,
    Aur,
    Binary,
}
```

### Manifest Manager

Handles reading, writing, and validating `zigroot.toml`:

```rust
pub struct Manifest {
    pub project: ProjectConfig,
    pub board: BoardConfig,
    pub packages: Vec<PackageRef>,
    pub build: BuildConfig,
    pub external: HashMap<String, ExternalArtifact>,
    pub cache: Option<CacheConfig>,
}
// NOTE: No [toolchain] section - toolchains are declared per-package

pub struct ProjectConfig {
    pub name: String,
    pub version: String,
}

pub struct BoardConfig {
    pub name: String,
    pub options: HashMap<String, OptionValue>,
}

pub struct BuildConfig {
    pub compress: bool,
    pub sandbox: bool,
    pub image_format: ImageFormat,
    pub rootfs_size: String,
}

impl Manifest {
    pub fn load(path: &Path) -> Result<Self>;
    pub fn save(&self, path: &Path) -> Result<()>;
    pub fn validate(&self) -> Result<Vec<ValidationError>>;
}
```

### Registry Client

Zigroot interacts with GitHub-hosted registries using the GitHub API and raw file access:

```rust
pub struct RegistryClient {
    http_client: reqwest::Client,
    cache_dir: PathBuf,
    package_registry: RegistryConfig,
    board_registry: RegistryConfig,
}

pub struct RegistryConfig {
    /// GitHub org/repo, e.g., "zigroot/zigroot-packages"
    pub repo: String,
    /// Branch to use, default "main"
    pub branch: String,
    /// Local cache TTL in seconds
    pub cache_ttl: u64,
}

impl RegistryClient {
    /// Fetch and cache the package index
    pub async fn fetch_package_index(&self) -> Result<PackageIndex>;
    
    /// Fetch and cache the board index
    pub async fn fetch_board_index(&self) -> Result<BoardIndex>;
    
    /// Get package metadata (shared across versions)
    pub async fn get_package_metadata(&self, name: &str) -> Result<PackageMetadata>;
    
    /// Get a specific package version
    pub async fn get_package_version(&self, name: &str, version: &str) -> Result<PackageVersion>;
    
    /// Get the full package definition (metadata + version merged)
    pub async fn get_package(&self, name: &str, version: Option<&str>) -> Result<PackageDefinition> {
        let metadata = self.get_package_metadata(name).await?;
        let version = match version {
            Some(v) => v.to_string(),
            None => self.get_latest_version(name).await?,
        };
        let ver_data = self.get_package_version(name, &version).await?;
        Ok(PackageDefinition::merge(metadata, ver_data))
    }
    
    /// Get a specific board definition
    pub async fn get_board(&self, name: &str) -> Result<BoardDefinition>;
    
    /// Search packages and boards
    pub async fn search(&self, query: &str) -> Result<SearchResults>;
    
    /// Force refresh of cached indexes
    pub async fn refresh(&self) -> Result<()>;
    
    /// Get latest version of a package from index
    async fn get_latest_version(&self, name: &str) -> Result<String>;
}
```

**Registry Interaction Flow:**

1. **Index Fetching** - On first use or when cache expires, fetch `index.json`:
   ```
   GET https://raw.githubusercontent.com/zigroot/zigroot-packages/main/index.json
   ```

2. **Package Metadata Fetching** - When package details needed:
   ```
   GET https://raw.githubusercontent.com/zigroot/zigroot-packages/main/packages/busybox/metadata.toml
   ```

3. **Version Fetching** - When a specific version is needed:
   ```
   GET https://raw.githubusercontent.com/zigroot/zigroot-packages/main/packages/busybox/1.36.1.toml
   ```

4. **Caching Strategy**:
   - Index cached locally in `~/.cache/zigroot/registry/`
   - Default TTL: 1 hour (configurable)
   - `--refresh` flag forces re-fetch
   - ETag/Last-Modified headers used for conditional requests

5. **Offline Mode**:
   - If network unavailable, use cached data
   - Warn user that data may be stale
   - Fail only if required data not in cache

**Index Format (auto-generated):**

```json
// zigroot-packages/index.json
{
  "version": 1,
  "updated": "2025-01-11T12:00:00Z",
  "packages": [
    {
      "name": "busybox",
      "description": "Swiss army knife of embedded Linux",
      "license": "GPL-2.0",
      "keywords": ["shell", "coreutils", "init"],
      "versions": [
        {"version": "1.36.1", "released": "2024-01-15", "sha256": "b8cc24..."},
        {"version": "1.36.0", "released": "2023-05-01", "sha256": "a1b2c3..."}
      ],
      "latest": "1.36.1"
    },
    {
      "name": "dropbear",
      "description": "Lightweight SSH server and client",
      "license": "MIT",
      "keywords": ["ssh", "network", "security"],
      "versions": [
        {"version": "2024.85", "released": "2024-03-20", "sha256": "def456..."}
      ],
      "latest": "2024.85"
    }
  ]
}
```

```json
// zigroot-boards/index.json
{
  "version": 1,
  "updated": "2025-01-11T12:00:00Z",
  "boards": [
    {
      "name": "luckfox-pico",
      "description": "Luckfox Pico (RV1103 SoC, Cortex-A7)",
      "arch": "arm",
      "target": "arm-linux-musleabihf",
      "keywords": ["rockchip", "rv1103", "camera"]
    },
    {
      "name": "raspberry-pi-4",
      "description": "Raspberry Pi 4 Model B",
      "arch": "aarch64",
      "target": "aarch64-linux-musl",
      "keywords": ["broadcom", "bcm2711", "sbc"]
    }
  ]
}
```

**Search Implementation:**

```rust
impl RegistryClient {
    pub async fn search(&self, query: &str) -> Result<SearchResults> {
        let pkg_index = self.fetch_package_index().await?;
        let board_index = self.fetch_board_index().await?;
        
        let query_lower = query.to_lowercase();
        
        let packages: Vec<_> = pkg_index.packages
            .iter()
            .filter(|p| {
                p.name.to_lowercase().contains(&query_lower) ||
                p.description.to_lowercase().contains(&query_lower) ||
                p.keywords.iter().any(|k| k.to_lowercase().contains(&query_lower))
            })
            .cloned()
            .collect();
        
        let boards: Vec<_> = board_index.boards
            .iter()
            .filter(|b| {
                b.name.to_lowercase().contains(&query_lower) ||
                b.description.to_lowercase().contains(&query_lower) ||
                b.keywords.iter().any(|k| k.to_lowercase().contains(&query_lower))
            })
            .cloned()
            .collect();
        
        Ok(SearchResults { packages, boards })
    }
}
```

### Package Definition

```rust
pub struct PackageDefinition {
    pub package: PackageMetadata,
    pub source: SourceConfig,
    pub build: BuildConfig,
    pub options: HashMap<String, OptionDefinition>,
}

pub struct PackageMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub arch: Option<Vec<String>>,
    pub provides: Option<Vec<String>>,
    pub conflicts: Option<Vec<String>>,
    pub depends: Option<Vec<Dependency>>,
    pub requires: Option<Vec<String>>,
    pub compress: Option<bool>,
    /// Minimum zigroot version required (semver constraint)
    pub zigroot_version: Option<String>,
}

/// Package source configuration.
/// A package MUST specify exactly ONE of: url+sha256, git+ref, or use [[sources]] array.
pub struct SourceConfig {
    /// Direct download URL for source tarball
    pub url: Option<String>,
    /// SHA256 checksum (required for url)
    pub sha256: Option<String>,
    /// Git repository URL (alternative to url)
    pub git: Option<String>,
    /// Git tag (with git)
    pub tag: Option<String>,
    /// Git branch (with git)
    pub branch: Option<String>,
    /// Git commit SHA (with git) - for reproducibility
    pub rev: Option<String>,
    /// Patches applied in alphabetical order (use 001-, 002- prefixes)
    pub patches: Option<Vec<String>>,
    /// Archive format (auto-detected from extension if not specified)
    pub format: Option<ArchiveFormat>,
    /// Subdirectory within archive/repo to use as source root
    pub subdir: Option<String>,
}

/// Multiple source files - used as [[sources]] array in TOML.
/// Alternative to [source] when package needs multiple source archives.
pub struct SourceFile {
    pub url: String,
    pub sha256: String,
    /// Destination filename (defaults to URL basename)
    pub filename: Option<String>,
}

/// Complete package definition combining source options
pub struct PackageSource {
    /// Single source (url or git)
    pub source: Option<SourceConfig>,
    /// Multiple source files (alternative to source)
    pub sources: Option<Vec<SourceFile>>,
}

pub enum ArchiveFormat {
    TarGz,
    TarBz2,
    TarXz,
    Zip,
    /// Raw file (no extraction needed)
    Raw,
}

pub enum BuildType {
    Autotools,
    Cmake,
    Meson,
    Make,
    Custom,
}

pub struct PackageBuildConfig {
    pub build_type: Option<BuildType>,
    pub steps: Option<Vec<BuildStep>>,
    /// Toolchain configuration for packages that can't use Zig
    pub toolchain: Option<ToolchainSpec>,
    pub configure_args: Option<Vec<String>>,
    pub make_args: Option<Vec<String>>,
    pub cmake_args: Option<Vec<String>>,
    pub network: bool,
    /// Pre-build hook script (runs before configure/build)
    pub pre_build: Option<String>,
    /// Post-build hook script (runs after build, before install)
    pub post_build: Option<String>,
}

/// Installation configuration
pub struct InstallConfig {
    /// Install type: auto (based on build type), custom (use script), declarative
    pub install_type: Option<InstallType>,
    /// Custom install script (for install_type = custom)
    pub script: Option<String>,
    /// Declarative install rules (for install_type = declarative)
    pub files: Option<Vec<InstallRule>>,
    /// Strip binaries (default: true for release builds)
    pub strip: Option<bool>,
    /// Default file permissions for installed files
    pub default_mode: Option<String>,
}

pub enum InstallType {
    /// Use default install based on build type (make install, etc.)
    Auto,
    /// Run custom install.sh script
    Custom,
    /// Use declarative file rules
    Declarative,
}

/// Declarative install rule
pub struct InstallRule {
    /// Source glob pattern (relative to build dir)
    pub src: String,
    /// Destination path (relative to rootfs)
    pub dst: String,
    /// File mode (e.g., "755" for executables)
    pub mode: Option<String>,
}

/// Toolchain specification - simple or explicit
pub enum ToolchainSpec {
    /// Use Zig (default, can be omitted)
    Zig,
    /// Use GCC with automatic URL resolution from bootlin.com
    /// Just specify the target and Zigroot figures out the rest
    Gcc {
        /// Target triple, e.g., "arm-linux-gnueabihf", "aarch64-linux-gnu"
        target: String,
        /// Optional: glibc version (default: "glibc")
        libc: Option<String>,
        /// Optional: toolchain release (default: "stable-2024.02-1")
        release: Option<String>,
    },
    /// Use GCC with explicit URLs per host platform (escape hatch)
    GccExplicit {
        /// Map of host platform to download URL
        /// e.g., { "linux-x86_64": "https://...", "darwin-aarch64": "https://..." }
        url: HashMap<String, String>,
    },
}

pub struct BuildStep {
    pub run: String,
    pub args: Vec<String>,
    /// Working directory (relative to srcdir)
    pub workdir: Option<String>,
    /// Environment variables for this step only
    pub env: Option<HashMap<String, String>>,
}

pub struct OptionDefinition {
    pub option_type: OptionType,
    pub default: OptionValue,
    pub description: String,
    pub choices: Option<Vec<String>>,
    pub pattern: Option<String>,
    pub allow_empty: Option<bool>,
    pub min: Option<i64>,
    pub max: Option<i64>,
}
```

### Board Definition

```rust
pub struct BoardDefinition {
    pub board: BoardMetadata,
    pub defaults: BoardDefaults,
    pub requires: Vec<String>,
    pub flash: Vec<FlashProfile>,
    pub options: HashMap<String, OptionDefinition>,
}

pub struct BoardMetadata {
    pub name: String,
    pub description: String,
    pub target: String,  // Zig target triple
    pub cpu: String,
    pub features: Option<Vec<String>>,
    pub kernel: Option<String>,
    /// Minimum zigroot version required (semver constraint)
    pub zigroot_version: Option<String>,
}

pub struct BoardDefaults {
    pub image_format: ImageFormat,
    pub rootfs_size: String,
    pub hostname: String,
}

pub struct FlashProfile {
    pub name: String,
    pub description: String,
    pub tool: Option<String>,
    pub script: Option<String>,
    pub requires: Vec<String>,  // External artifact names
}
```

### Dependency Resolver

```rust
pub struct DependencyResolver {
    packages: HashMap<String, PackageDefinition>,
}

impl DependencyResolver {
    /// Compute build order using topological sort
    pub fn resolve_build_order(&self, requested: &[String]) -> Result<Vec<String>>;
    
    /// Detect circular dependencies
    pub fn detect_cycles(&self) -> Option<Vec<String>>;
    
    /// Resolve version constraints
    pub fn resolve_versions(&self, constraints: &[Dependency]) -> Result<HashMap<String, Version>>;
}
```

### Build Orchestrator

```rust
pub struct BuildOrchestrator {
    manifest: Manifest,
    board: BoardDefinition,
    zig_toolchain: ZigToolchain,      // Always available - built into Zig
    gcc_cache: GccToolchainCache,     // Downloaded on-demand per package
    sandbox: Option<DockerSandbox>,
    cache: BuildCache,
    progress: ProgressReporter,
}

/// Zig toolchain - zero setup, always available
pub struct ZigToolchain {
    pub zig_path: PathBuf,  // Path to zig binary
}

/// Cache of downloaded GCC toolchains (keyed by URL hash)
pub struct GccToolchainCache {
    pub cache_dir: PathBuf,
}

impl GccToolchainCache {
    /// Get or download a GCC toolchain for the given URL
    pub async fn get_toolchain(&self, url: &str, target: &str) -> Result<GccToolchain>;
}

/// A specific GCC toolchain instance
pub struct GccToolchain {
    pub path: PathBuf,      // Path to extracted toolchain
    pub prefix: String,     // e.g., "arm-linux-gnueabihf-"
}

impl BuildOrchestrator {
    pub async fn build_all(&self) -> Result<BuildResult>;
    pub async fn build_package(&self, name: &str) -> Result<PackageBuildResult>;
    
    fn should_rebuild(&self, package: &str) -> bool;
    fn setup_build_env(&self, package: &PackageDefinition) -> BuildEnvironment;
    fn execute_build_steps(&self, package: &PackageDefinition, env: &BuildEnvironment) -> Result<()>;
    
    /// Select toolchain based on package requirements
    async fn get_toolchain(&self, package: &PackageDefinition) -> Result<Box<dyn Toolchain>> {
        match &package.build.toolchain {
            None | Some(ToolchainSpec::Zig) => {
                Ok(Box::new(self.zig_toolchain.clone()))  // Default: Zig
            }
            Some(ToolchainSpec::Gcc { target, libc, release }) => {
                // Auto-resolve bootlin URL based on host platform
                let host = detect_host_platform();
                let url = resolve_bootlin_url(&host, target, libc, release)?;
                let gcc = self.gcc_cache.get_toolchain(&url, target).await?;
                Ok(Box::new(gcc))
            }
            Some(ToolchainSpec::GccExplicit { url }) => {
                // Use explicit URL for current host
                let host = detect_host_platform();
                let toolchain_url = url.get(&host)
                    .ok_or_else(|| BuildError::UnsupportedHostPlatform { 
                        host: host.clone(),
                        supported: url.keys().cloned().collect(),
                    })?;
                let gcc = self.gcc_cache.get_toolchain(toolchain_url, &self.board.target).await?;
                Ok(Box::new(gcc))
            }
        }
    }

    /// Detect current host platform (os-arch)
    fn detect_host_platform() -> String {
        let os = std::env::consts::OS;      // "linux", "macos", "windows"
        let arch = std::env::consts::ARCH;  // "x86_64", "aarch64"
        let os_name = match os {
            "macos" => "darwin",
            other => other,
        };
        format!("{}-{}", os_name, arch)
    }
    
    /// Resolve bootlin.com toolchain URL for given host and target
    fn resolve_bootlin_url(
        host: &str, 
        target: &str, 
        libc: &Option<String>,
        release: &Option<String>,
    ) -> Result<String> {
        let libc = libc.as_deref().unwrap_or("glibc");
        let release = release.as_deref().unwrap_or("stable-2024.02-1");
        
        // Map target triple to bootlin architecture name
        let bootlin_arch = match target {
            "arm-linux-gnueabihf" => "armv7-eabihf",
            "aarch64-linux-gnu" => "aarch64",
            "x86_64-linux-gnu" => "x86-64",
            "riscv64-linux-gnu" => "riscv64-lp64d",
            other => return Err(BuildError::UnsupportedTarget { target: other.to_string() }),
        };
        
        // Bootlin provides Linux-hosted toolchains only
        // For macOS, we'd need a different source or Docker
        match host {
            "linux-x86_64" | "linux-aarch64" => {
                Ok(format!(
                    "https://toolchains.bootlin.com/downloads/releases/toolchains/{}/tarballs/{}--{}--{}.tar.bz2",
                    bootlin_arch, bootlin_arch, libc, release
                ))
            }
            _ => Err(BuildError::BootlinNotAvailable { 
                host: host.to_string(),
                suggestion: "Use [build.toolchain.url] with explicit URLs for this platform, or build in Docker".to_string(),
            }),
        }
    }
}

/// Build environment for a package.
/// 
/// For Zig-based builds (the default), cross-compilation is handled internally
/// by Zig - we just set CC="zig cc -target <target>" and Zig handles everything.
/// No sysroot, no separate AR/LD/RANLIB needed.
///
/// For GCC-based builds (kernel, bootloader), we set up the traditional
/// cross-compilation environment with the downloaded toolchain.
pub struct BuildEnvironment {
    /// Compiler command (e.g., "zig cc -target arm-linux-musleabihf" or "arm-linux-gnueabihf-gcc")
    pub cc: String,
    /// C++ compiler (e.g., "zig c++ -target arm-linux-musleabihf" or "arm-linux-gnueabihf-g++")
    pub cxx: String,
    /// Archiver - only set for GCC builds, Zig handles this internally
    pub ar: Option<String>,
    /// Linker - only set for GCC builds
    pub ld: Option<String>,
    /// Target triple (Zig format: arm-linux-musleabihf)
    pub target: String,
    /// CPU model for optimization flags
    pub cpu: String,
    /// Source directory (extracted package sources)
    pub srcdir: PathBuf,
    /// Destination directory for installed files (staging area)
    pub destdir: PathBuf,
    /// Install prefix (usually /usr)
    pub prefix: String,
    /// Number of parallel jobs
    pub jobs: usize,
    /// Paths to dependency build outputs: ZIGROOT_PKG_<NAME>_DIR
    pub pkg_dirs: HashMap<String, PathBuf>,
    /// Package option values: ZIGROOT_OPT_<NAME>
    pub options: HashMap<String, String>,
    /// Board option values: ZIGROOT_BOARD_OPT_<NAME>
    pub board_options: HashMap<String, String>,
    /// Whether this is a Zig or GCC build
    pub toolchain_type: ToolchainType,
}

pub enum ToolchainType {
    /// Zig handles cross-compilation internally - minimal env setup needed
    Zig,
    /// Traditional GCC cross-toolchain - full env setup
    Gcc,
}

impl BuildEnvironment {
    /// Create environment for Zig-based build (default)
    pub fn for_zig(target: &str, cpu: &str, srcdir: PathBuf, destdir: PathBuf) -> Self {
        Self {
            cc: format!("zig cc -target {}", target),
            cxx: format!("zig c++ -target {}", target),
            ar: None,  // Zig handles archiving internally
            ld: None,  // Zig handles linking internally
            target: target.to_string(),
            cpu: cpu.to_string(),
            srcdir,
            destdir,
            prefix: "/usr".to_string(),
            jobs: num_cpus::get(),
            pkg_dirs: HashMap::new(),
            options: HashMap::new(),
            board_options: HashMap::new(),
            toolchain_type: ToolchainType::Zig,
        }
    }
    
    /// Create environment for GCC-based build (kernel, bootloader)
    pub fn for_gcc(toolchain: &GccToolchain, target: &str, cpu: &str, srcdir: PathBuf, destdir: PathBuf) -> Self {
        let prefix = &toolchain.prefix;  // e.g., "arm-linux-gnueabihf-"
        Self {
            cc: format!("{}gcc", prefix),
            cxx: format!("{}g++", prefix),
            ar: Some(format!("{}ar", prefix)),
            ld: Some(format!("{}ld", prefix)),
            target: target.to_string(),
            cpu: cpu.to_string(),
            srcdir,
            destdir,
            prefix: "/usr".to_string(),
            jobs: num_cpus::get(),
            pkg_dirs: HashMap::new(),
            options: HashMap::new(),
            board_options: HashMap::new(),
            toolchain_type: ToolchainType::Gcc,
        }
    }
    
    /// Export as environment variables for build scripts
    pub fn to_env_vars(&self) -> HashMap<String, String> {
        let mut env = HashMap::new();
        
        // Core compiler vars
        env.insert("CC".to_string(), self.cc.clone());
        env.insert("CXX".to_string(), self.cxx.clone());
        
        // Only set AR/LD for GCC builds
        if let Some(ar) = &self.ar {
            env.insert("AR".to_string(), ar.clone());
        }
        if let Some(ld) = &self.ld {
            env.insert("LD".to_string(), ld.clone());
        }
        
        // Build paths
        env.insert("TARGET".to_string(), self.target.clone());
        env.insert("CPU".to_string(), self.cpu.clone());
        env.insert("SRCDIR".to_string(), self.srcdir.display().to_string());
        env.insert("DESTDIR".to_string(), self.destdir.display().to_string());
        env.insert("PREFIX".to_string(), self.prefix.clone());
        env.insert("JOBS".to_string(), self.jobs.to_string());
        
        // Dependency paths
        for (name, path) in &self.pkg_dirs {
            env.insert(
                format!("ZIGROOT_PKG_{}_DIR", name.to_uppercase().replace('-', "_")),
                path.display().to_string(),
            );
        }
        
        // Package options
        for (name, value) in &self.options {
            env.insert(
                format!("ZIGROOT_OPT_{}", name.to_uppercase().replace('-', "_")),
                value.clone(),
            );
        }
        
        // Board options
        for (name, value) in &self.board_options {
            env.insert(
                format!("ZIGROOT_BOARD_OPT_{}", name.to_uppercase().replace('-', "_")),
                value.clone(),
            );
        }
        
        env
    }
}
```

### Docker Sandbox

```rust
pub struct DockerSandbox {
    client: DockerClient,
    image: String,
}

impl DockerSandbox {
    pub fn new() -> Result<Self>;
    
    /// Run build inside container
    pub async fn run_build(
        &self,
        package: &PackageDefinition,
        env: &BuildEnvironment,
        allow_network: bool,
    ) -> Result<()>;
    
    /// Extract build artifacts from container
    pub async fn extract_artifacts(&self, container_id: &str, dest: &Path) -> Result<()>;
}

// Docker interaction via CLI for simplicity
impl DockerSandbox {
    async fn docker_run(&self, args: &[&str]) -> Result<String> {
        Command::new("docker")
            .args(args)
            .output()
            .await
    }
}
```

## Local Data Storage

Zigroot stores cached data and settings in platform-specific directories following XDG conventions on Linux and standard locations on macOS/Windows:

### Directory Structure

| Platform | Cache Dir | Config Dir | Data Dir |
|----------|-----------|------------|----------|
| Linux | `~/.cache/zigroot/` | `~/.config/zigroot/` | `~/.local/share/zigroot/` |
| macOS | `~/Library/Caches/zigroot/` | `~/Library/Application Support/zigroot/` | `~/Library/Application Support/zigroot/` |
| Windows | `%LOCALAPPDATA%\zigroot\cache\` | `%APPDATA%\zigroot\` | `%LOCALAPPDATA%\zigroot\data\` |

### Cache Directory (`~/.cache/zigroot/`)

Temporary data that can be safely deleted:

```
~/.cache/zigroot/
├── registry/
│   ├── packages-index.json      # Cached package index (TTL: 1 hour)
│   ├── boards-index.json        # Cached board index (TTL: 1 hour)
│   └── packages/                # Cached package/board definitions
│       ├── busybox/
│       │   ├── metadata.toml
│       │   └── 1.36.1.toml
│       └── ...
├── toolchains/                  # Downloaded GCC toolchains (keyed by URL hash)
│   ├── abc123.tar.bz2           # Compressed toolchain archive
│   └── abc123/                  # Extracted toolchain
│       └── arm-linux-gnueabihf/
└── update-check.json            # Last update check result
```

### Config Directory (`~/.config/zigroot/`)

User configuration (persisted across reinstalls):

```
~/.config/zigroot/
├── config.toml                  # Global user settings
└── credentials.toml             # GitHub tokens (encrypted)
```

**Example config.toml:**

```toml
# Global zigroot configuration
# These settings apply to all projects unless overridden

[registry]
# Custom registry URLs (default: GitHub zigroot org)
# packages = "https://github.com/myorg/my-packages"
# boards = "https://github.com/myorg/my-boards"

# Cache TTL in seconds (default: 3600 = 1 hour)
cache_ttl = 3600

[build]
# Default number of parallel jobs (default: CPU count)
# jobs = 4

# Default sandbox setting (default: false)
# sandbox = false

[update]
# Check for zigroot updates automatically (default: true)
auto_check = true

# How often to check for updates in seconds (default: 86400 = 1 day)
check_interval = 86400

[output]
# Color output (auto, always, never)
color = "auto"

# Default verbosity (0 = normal, 1 = verbose, 2 = debug)
verbosity = 0
```

### Data Directory (`~/.local/share/zigroot/`)

Persistent data that should survive cache clears:

```
~/.local/share/zigroot/
├── downloads/                   # Shared source archive cache
│   ├── busybox-1.36.1.tar.bz2
│   └── ...
└── build-cache/                 # Content-addressable build cache
    ├── index.json               # Cache index
    └── objects/                 # Cached build artifacts (by hash)
        ├── abc123.tar.gz
        └── ...
```

### Update Check Cache

The update check result is cached to avoid repeated network requests:

```json
// ~/.cache/zigroot/update-check.json
{
  "checked_at": "2025-01-11T12:00:00Z",
  "current_version": "0.1.0",
  "latest_version": "0.2.0",
  "release_url": "https://github.com/zigroot/zigroot-cli/releases/tag/v0.2.0",
  "has_update": true
}
```

### Environment Variables

Users can override default directories:

| Variable | Purpose | Default |
|----------|---------|---------|
| `ZIGROOT_CACHE_DIR` | Cache directory | Platform-specific |
| `ZIGROOT_CONFIG_DIR` | Config directory | Platform-specific |
| `ZIGROOT_DATA_DIR` | Data directory | Platform-specific |
| `ZIGROOT_LOG` | Log level (error, warn, info, debug, trace) | warn |
| `GITHUB_TOKEN` | GitHub API token for publishing | - |

## Configuration File Syntax Summary

This section provides a quick reference for all configuration file formats used by zigroot.

### File Overview

| File | Location | Purpose |
|------|----------|---------|
| `zigroot.toml` | Project root | Project manifest - packages, board, build settings |
| `zigroot.lock` | Project root | Lock file - exact versions for reproducibility |
| `package.toml` | `packages/<name>/` | Local package definition |
| `metadata.toml` | Registry `packages/<name>/` | Registry package metadata (shared across versions) |
| `<version>.toml` | Registry `packages/<name>/` | Registry package version-specific data (`[release]` + `[source]`) |
| `board.toml` | `boards/<name>/` or registry | Board definition |
| `config.toml` | `~/.config/zigroot/` | Global user settings |

### Common Patterns

**Options (used in packages and boards):**
```toml
[options.<name>]
type = "bool" | "string" | "choice" | "number"
default = <value>
description = "..."
# For choice:
choices = ["a", "b", "c"]
# For string:
pattern = "regex"
allow_empty = true | false
# For number:
min = 0
max = 100
```

**Source specification (packages):**

A package MUST specify exactly ONE of these source types:

```toml
# Option 1: URL tarball (most common)
[source]
url = "https://example.com/foo-1.0.tar.gz"
sha256 = "abc123..."
subdir = "foo-1.0"  # Optional: subdirectory within archive

# Option 2: Git repository
[source]
git = "https://github.com/user/repo"
tag = "v1.0"      # Use tag, branch, OR rev
# branch = "main"
# rev = "abc123"
subdir = "packages/core"  # Optional: subdirectory within repo

# Option 3: Multiple source files (for complex packages)
[[sources]]
url = "https://example.com/main.tar.gz"
sha256 = "abc123..."

[[sources]]
url = "https://example.com/extra.tar.gz"
sha256 = "def456..."
filename = "extra-data.tar.gz"  # Optional: rename on download
```

**Patches (applied to any source type):**
```toml
[source]
url = "..."
sha256 = "..."
patches = ["patches/001-fix.patch", "patches/002-feature.patch"]
```

**Dependencies (packages):**
```toml
# In package.toml or metadata.toml - version constraints
depends = ["zlib>=1.2.11", "openssl^3.0"]  # Build-time dependencies
requires = ["busybox"]                      # Runtime dependencies (no version)

# In zigroot.lock - resolved exact versions
depends = ["zlib@1.2.13", "openssl@3.2.0"]
```

**Build section (note: different meaning in different files):**
```toml
# In zigroot.toml - project-level build settings
[build]
compress = true
sandbox = false
image_format = "ext4"
rootfs_size = "64M"

# In package.toml - package build instructions
[build]
type = "autotools"  # autotools, cmake, meson, make, custom
configure_args = ["--disable-nls"]
make_args = ["CROSS_COMPILE=${TARGET}-"]
```

**External artifacts (zigroot.toml):**
```toml
[external.<name>]
type = "bootloader" | "kernel" | "partition_table" | "dtb" | "firmware" | "other"
url = "https://..."  # Remote
sha256 = "..."       # Required for url
path = "local/file"  # Local (no sha256 needed)
format = "gpt" | "mbr" | "rockchip"  # For partition_table only
```

**Package references (zigroot.toml):**
```toml
[packages]
# Simple version (default registry)
busybox = "1.36.1"

# Local path
myapp = { path = "packages/myapp" }

# Git repository
forked = { git = "https://...", tag = "v1.0" }
forked = { git = "https://...", branch = "main" }
forked = { git = "https://...", rev = "abc123" }

# Custom registry
private = { registry = "https://...", version = "1.0" }
```

**Package source in lock file:**
```toml
[[package]]
name = "..."
version = "..."
sha256 = "..."
source = "registry"                    # Default, can be omitted
source = "path:packages/myapp"         # Local
source = "git:https://...#v1.0"        # Git with ref
source = "registry:https://..."        # Custom registry
```

**Board definition (board.toml):**
```toml
[board]
name = "board-name"
description = "..."
target = "arm-linux-musleabihf"  # Zig target triple
cpu = "cortex_a7"
features = ["neon", "vfpv4"]     # Optional
kernel = "linux-board"           # Optional: default kernel package
zigroot_version = ">=0.2.0"      # Optional: minimum zigroot version

[defaults]
image_format = "ext4"            # ext4, squashfs, initramfs
rootfs_size = "32M"
hostname = "mydevice"

requires = ["base", "busybox"]

[options.<name>]
type = "choice"
default = "sd"
choices = ["sd", "emmc"]
description = "..."

[[flash]]
name = "method-name"
description = "..."
tool = "rkflashtool"             # External tool
script = "flash.sh"              # Script to run
requires = ["bootloader"]        # External artifacts needed
```

## Command Flows

This section describes the implementation flow for each command using Mermaid flowcharts and pseudocode.

### init

**Purpose:** Create a new zigroot project with proper structure.

```mermaid
flowchart TD
    A[zigroot init] --> B{Directory empty?}
    B -->|No| C{--force flag?}
    C -->|No| D[Error: Directory not empty]
    C -->|Yes| E[Continue with init]
    B -->|Yes| E
    E --> F{--board specified?}
    F -->|Yes| G[Fetch board from registry]
    G --> H{Board found?}
    H -->|No| I[Error: Board not found]
    H -->|Yes| J[Store board config]
    F -->|No| K[Use default board placeholder]
    J --> L[Generate zigroot.toml]
    K --> L
    L --> M[Create directory structure]
    M --> N[Generate .gitignore]
    N --> O{.gitignore exists?}
    O -->|Yes| P[Append zigroot entries]
    O -->|No| Q[Create new .gitignore]
    P --> R[Display success message]
    Q --> R
```

**Pseudocode:**
```
function init(args: InitArgs) -> Result:
    // 1. Validate directory state
    if not is_empty(current_dir) and not args.force:
        return Error("Directory is not empty. Use --force to override.")
    
    // 2. Fetch board if specified
    board_config = None
    if args.board:
        board = registry.fetch_board(args.board)
        if not board:
            return Error(f"Board '{args.board}' not found. Run 'zigroot board list' to see available boards.")
        board_config = board
    
    // 3. Generate zigroot.toml from template
    manifest = generate_manifest_template(
        project_name = dirname(current_dir),
        board = board_config
    )
    write_file("zigroot.toml", manifest)
    
    // 4. Create directory structure
    create_dir("packages")
    create_dir("boards")
    create_dir("user/files")
    create_dir("user/scripts")
    
    // 5. Handle .gitignore
    gitignore_content = GITIGNORE_TEMPLATE
    if exists(".gitignore"):
        existing = read_file(".gitignore")
        if "# zigroot" not in existing:
            append_file(".gitignore", "\n# zigroot\n" + gitignore_content)
    else:
        write_file(".gitignore", gitignore_content)
    
    // 6. Success output
    print("✓ Created zigroot.toml")
    print("✓ Created project structure")
    if board_config:
        print(f"✓ Configured for board: {board_config.name}")
    print("\nNext steps:")
    print("  zigroot add <package>    Add packages to your project")
    print("  zigroot build            Build the rootfs image")
```

**Errors:**
| Condition | Message | Exit Code |
|-----------|---------|-----------|
| Non-empty directory | "Directory is not empty. Use --force to override." | 2 |
| Board not found | "Board 'X' not found. Run 'zigroot board list' to see available boards." | 2 |
| Network error fetching board | "Failed to fetch board 'X': {error}. Check your network connection." | 4 |
| Permission denied | "Cannot create files: permission denied" | 1 |

---

### add

**Purpose:** Add a package to the project manifest.

```mermaid
flowchart TD
    A[zigroot add pkg] --> B{Parse package spec}
    B --> C{Source type?}
    C -->|Registry| D[Fetch from registry]
    C -->|--git| E[Validate git URL]
    C -->|--path| F[Validate local path]
    C -->|--registry| G[Fetch from custom registry]
    
    D --> H{Package found?}
    H -->|No| I[Error: Package not found]
    H -->|Yes| J[Resolve version]
    
    E --> K{Valid git URL?}
    K -->|No| L[Error: Invalid git URL]
    K -->|Yes| M[Validate ref exists]
    M --> N{Ref valid?}
    N -->|No| O[Error: Ref not found]
    N -->|Yes| P[Add git source to manifest]
    
    F --> Q{Path exists?}
    Q -->|No| R[Error: Path not found]
    Q -->|Yes| S{Valid package.toml?}
    S -->|No| T[Error: Invalid package]
    S -->|Yes| U[Add path source to manifest]
    
    J --> V[Resolve dependencies]
    V --> W{Conflicts?}
    W -->|Yes| X[Error: Dependency conflict]
    W -->|No| Y[Add to manifest]
    
    P --> Z[Update lock file]
    U --> Z
    Y --> Z
    G --> H
    
    Z --> AA[Display success]
```

**Pseudocode:**
```
function add(args: AddArgs) -> Result:
    // 1. Load existing manifest
    manifest = load_manifest("zigroot.toml")
    
    // 2. Parse package specification
    if args.git:
        // Git source
        ref = args.tag or args.branch or args.rev
        if not ref:
            return Error("Git source requires --tag, --branch, or --rev")
        
        package_ref = PackageRef::Git {
            url: args.git,
            tag: args.tag,
            branch: args.branch,
            rev: args.rev,
        }
        
        // Validate git ref exists (optional, can be slow)
        if args.branch:
            resolved_sha = git_resolve_ref(args.git, args.branch)
            // Store SHA in lock file for reproducibility
    
    elif args.path:
        // Local path
        if not exists(args.path):
            return Error(f"Path '{args.path}' does not exist")
        if not exists(args.path / "package.toml"):
            return Error(f"No package.toml found in '{args.path}'")
        
        pkg_def = parse_package(args.path / "package.toml")
        package_ref = PackageRef::Path { path: args.path }
    
    elif args.registry:
        // Custom registry
        pkg = fetch_from_registry(args.registry, args.package)
        package_ref = PackageRef::Registry {
            url: args.registry,
            version: pkg.version,
        }
    
    else:
        // Default registry
        (name, version) = parse_package_spec(args.package)  // "busybox@1.36.1" -> ("busybox", "1.36.1")
        
        pkg = registry.fetch_package(name, version)
        if not pkg:
            suggestions = registry.search(name)
            return Error(f"Package '{name}' not found. Did you mean: {suggestions}?")
        
        package_ref = PackageRef::Version(pkg.version)
    
    // 3. Resolve dependencies
    all_deps = resolve_dependencies(manifest.packages + [package_ref])
    if conflicts := detect_conflicts(all_deps):
        return Error(f"Dependency conflict: {format_conflicts(conflicts)}")
    
    // 4. Update manifest
    manifest.packages[name] = package_ref
    save_manifest(manifest)
    
    // 5. Update lock file
    lock = load_or_create_lock()
    lock.packages.extend(all_deps)
    save_lock(lock)
    
    // 6. Success output
    print(f"✓ Added {name}@{version}")
    if new_deps:
        print(f"  Also added {len(new_deps)} dependencies")
```

**Errors:**
| Condition | Message | Exit Code |
|-----------|---------|-----------|
| Package not found | "Package 'X' not found. Did you mean: Y, Z?" | 2 |
| Invalid version | "Version 'X' not found for package 'Y'. Available: 1.0, 2.0" | 2 |
| Git ref not found | "Git ref 'X' not found in repository" | 2 |
| Path not found | "Path 'X' does not exist" | 2 |
| Invalid package.toml | "Invalid package.toml: missing field 'name'" | 2 |
| Dependency conflict | "Dependency conflict: X requires Y>=2.0, but Z requires Y<2.0" | 5 |
| Network error | "Failed to fetch package: {error}" | 4 |

---

### fetch

**Purpose:** Download all package sources for offline building.

```mermaid
flowchart TD
    A[zigroot fetch] --> B[Load manifest + lock]
    B --> C[Collect all packages]
    C --> D[Filter already downloaded]
    D --> E{--force flag?}
    E -->|Yes| F[Download all]
    E -->|No| G[Download missing only]
    
    F --> H[Start parallel downloads]
    G --> H
    
    H --> I[For each package]
    I --> J{Source type?}
    J -->|URL| K[Download archive]
    J -->|Git| L[Clone repository]
    J -->|Path| M[Skip - local]
    
    K --> N[Verify SHA256]
    N --> O{Checksum OK?}
    O -->|No| P[Delete + retry]
    P --> Q{Retries left?}
    Q -->|No| R[Error: Checksum failed]
    Q -->|Yes| K
    O -->|Yes| S[Mark complete]
    
    L --> T[Checkout ref]
    T --> S
    M --> S
    
    S --> U{More packages?}
    U -->|Yes| I
    U -->|No| V[Fetch external artifacts]
    V --> W[Display summary]
```

**Pseudocode:**
```
function fetch(args: FetchArgs) -> Result:
    // 1. Load manifest and lock file
    manifest = load_manifest("zigroot.toml")
    lock = load_lock("zigroot.lock")
    
    // 2. Collect packages to download
    packages = resolve_all_packages(manifest, lock)
    
    // 3. Filter based on existing downloads
    to_download = []
    for pkg in packages:
        if pkg.source is Path:
            continue  // Local packages don't need download
        
        dest = downloads_dir / pkg.archive_name
        if exists(dest) and not args.force:
            if verify_checksum(dest, pkg.sha256):
                continue  // Already downloaded and valid
            else:
                delete(dest)  // Corrupted, re-download
        
        to_download.append(pkg)
    
    // 4. Download in parallel
    parallelism = args.parallel or num_cpus()
    progress = MultiProgress::new()
    
    results = parallel_map(to_download, parallelism, |pkg| {
        bar = progress.add_bar(pkg.name)
        
        for attempt in 1..=MAX_RETRIES:
            try:
                if pkg.source is Git:
                    git_clone(pkg.git_url, pkg.ref, downloads_dir / pkg.name)
                else:
                    download_with_progress(pkg.url, dest, bar)
                    
                    if not verify_checksum(dest, pkg.sha256):
                        delete(dest)
                        raise ChecksumError(pkg.name, expected, actual)
                
                return Ok(pkg.name)
            except DownloadError as e:
                if attempt == MAX_RETRIES:
                    return Err(e)
                sleep(exponential_backoff(attempt))
    })
    
    // 5. Fetch external artifacts
    for artifact in manifest.external.values():
        if artifact.url and not exists(external_dir / artifact.name):
            download(artifact.url, external_dir / artifact.name)
            verify_checksum(external_dir / artifact.name, artifact.sha256)
    
    // 6. Report results
    succeeded = results.filter(|r| r.is_ok()).count()
    failed = results.filter(|r| r.is_err())
    
    print(f"✓ Downloaded {succeeded} packages")
    if failed:
        for err in failed:
            print(f"✗ {err}")
        return Error("Some downloads failed")
```

**Errors:**
| Condition | Message | Exit Code |
|-----------|---------|-----------|
| Checksum mismatch | "Checksum mismatch for 'X': expected abc123, got def456" | 4 |
| Download failed | "Failed to download 'X' after 3 attempts: {error}" | 4 |
| Git clone failed | "Failed to clone 'X': {error}" | 4 |
| Network unreachable | "Network error: unable to reach {url}" | 4 |

---

### build

**Purpose:** Compile all packages and create the rootfs image.

```mermaid
flowchart TD
    A[zigroot build] --> B[Load manifest + lock + board]
    B --> C[Resolve build order]
    C --> D{Circular deps?}
    D -->|Yes| E[Error: Circular dependency]
    D -->|No| F[Check sources downloaded]
    
    F --> G{All sources ready?}
    G -->|No| H[Run fetch first]
    H --> F
    G -->|Yes| I[Setup build environment]
    
    I --> J[For each package in order]
    J --> K{Needs rebuild?}
    K -->|No| L[Skip - cached]
    K -->|Yes| M[Extract source]
    
    M --> N[Apply patches]
    N --> O{Toolchain type?}
    O -->|Zig| P[Setup Zig env]
    O -->|GCC| Q[Download/setup GCC]
    
    P --> R[Run build steps]
    Q --> R
    R --> S{Build success?}
    S -->|No| T[Error: Build failed]
    S -->|Yes| U[Run install]
    
    U --> V[Copy to staging rootfs]
    V --> W{More packages?}
    W -->|Yes| J
    W -->|No| X[Run post-build script]
    
    X --> Y[Copy user files]
    Y --> Z{Compress binaries?}
    Z -->|Yes| AA[Run UPX]
    Z -->|No| AB[Skip compression]
    
    AA --> AC[Create image]
    AB --> AC
    AC --> AD[Display summary]
```

**Pseudocode:**
```
function build(args: BuildArgs) -> Result:
    // 1. Load configuration
    manifest = load_manifest("zigroot.toml")
    lock = load_lock("zigroot.lock")
    board = load_board(manifest.board.name)
    
    // 2. Resolve build order
    packages = if args.package:
        [find_package(args.package)]
    else:
        resolve_all_packages(manifest, lock)
    
    build_order = topological_sort(packages)
    if cycle := detect_cycle(packages):
        return Error(f"Circular dependency: {' -> '.join(cycle)}")
    
    // 3. Check sources are downloaded
    missing = packages.filter(|p| not source_exists(p))
    if missing:
        print("Downloading missing sources...")
        fetch(FetchArgs { packages: missing })
    
    // 4. Setup build environment
    build_dir = "build"
    rootfs_dir = build_dir / "rootfs"
    create_dir(rootfs_dir)
    
    // 5. Build each package
    start_time = now()
    built_count = 0
    
    for pkg in build_order:
        // Check if rebuild needed
        if not args.force and is_cached(pkg):
            print(f"  {pkg.name} (cached)")
            continue
        
        print(f"Building {pkg.name}...")
        
        // Extract source
        src_dir = build_dir / "src" / pkg.name
        extract(downloads_dir / pkg.archive, src_dir)
        
        // Apply patches
        for patch in pkg.patches:
            apply_patch(src_dir, patch)
        
        // Setup toolchain
        env = match pkg.build.toolchain:
            Zig => BuildEnvironment::for_zig(board.target, board.cpu, src_dir)
            Gcc(spec) => 
                toolchain = gcc_cache.get_or_download(spec)
                BuildEnvironment::for_gcc(toolchain, board.target, board.cpu, src_dir)
        
        // Add dependency paths
        for dep in pkg.depends:
            env.pkg_dirs[dep] = build_dir / "pkg" / dep
        
        // Add options
        for (name, value) in resolve_options(pkg, manifest):
            env.options[name] = value
        
        // Run build
        pkg_build_dir = build_dir / "pkg" / pkg.name
        env.destdir = pkg_build_dir
        
        log_file = build_dir / "logs" / f"{pkg.name}.log"
        result = run_build(pkg, env, log_file)
        
        if result.is_err():
            print(f"✗ Build failed for {pkg.name}")
            print(f"  See: {log_file}")
            return result
        
        // Install to rootfs
        install_to_rootfs(pkg, pkg_build_dir, rootfs_dir)
        
        // Update cache stamp
        update_stamp(pkg)
        built_count += 1
    
    // 6. Post-build
    if exists("user/scripts/post-build.sh"):
        run_script("user/scripts/post-build.sh", env={"ROOTFS": rootfs_dir})
    
    // 7. Copy user files
    if exists("user/files"):
        copy_tree("user/files", rootfs_dir)
    
    // 8. Compress binaries
    compress = args.compress or (not args.no_compress and manifest.build.compress)
    if compress:
        compress_binaries(rootfs_dir, board.target)
    
    // 9. Create image
    image_format = manifest.build.image_format or board.defaults.image_format
    image_size = manifest.build.rootfs_size or board.defaults.rootfs_size
    output_path = "output" / f"rootfs.{image_format}"
    
    create_image(rootfs_dir, output_path, image_format, image_size)
    
    // 10. Summary
    elapsed = now() - start_time
    image_size_mb = file_size(output_path) / 1024 / 1024
    
    print(f"\n✓ Build complete in {elapsed}")
    print(f"  Packages built: {built_count}")
    print(f"  Image: {output_path} ({image_size_mb:.1f} MB)")
```

**Errors:**
| Condition | Message | Exit Code |
|-----------|---------|-----------|
| Circular dependency | "Circular dependency detected: A -> B -> C -> A" | 5 |
| Build failed | "Build failed for 'X'. See build/logs/X.log" | 3 |
| Image too large | "Rootfs size (128MB) exceeds configured limit (64MB)" | 3 |
| Missing toolchain | "GCC toolchain not available for this host platform" | 3 |
| Post-build script failed | "Post-build script failed with exit code X" | 3 |

---

### flash

**Purpose:** Flash the built image to a device.

```mermaid
flowchart TD
    A[zigroot flash] --> B{Method specified?}
    B -->|No| C[List available methods]
    B -->|Yes| D[Load board flash profiles]
    
    D --> E{Method exists?}
    E -->|No| F[Error: Unknown method]
    E -->|Yes| G[Check required artifacts]
    
    G --> H{All artifacts present?}
    H -->|No| I{URLs available?}
    I -->|Yes| J[Download missing artifacts]
    I -->|No| K[Error: Missing artifacts]
    J --> L[Verify checksums]
    H -->|Yes| L
    
    L --> M[Check required tools]
    M --> N{Tools installed?}
    N -->|No| O[Error: Missing tools]
    N -->|Yes| P{--yes flag?}
    
    P -->|No| Q[Show confirmation prompt]
    Q --> R{User confirms?}
    R -->|No| S[Abort]
    R -->|Yes| T[Execute flash]
    P -->|Yes| T
    
    T --> U{Flash success?}
    U -->|No| V[Error: Flash failed]
    U -->|Yes| W[Display success]
```

**Pseudocode:**
```
function flash(args: FlashArgs) -> Result:
    // 1. Load board and flash profiles
    manifest = load_manifest("zigroot.toml")
    board = load_board(manifest.board.name)
    
    // 2. Handle --list or no method
    if args.list or not args.method:
        print("Available flash methods:")
        for profile in board.flash:
            print(f"  {profile.name}: {profile.description}")
        if not args.method:
            print("\nUsage: zigroot flash <method>")
        return Ok()
    
    // 3. Find flash profile
    profile = board.flash.find(|p| p.name == args.method)
    if not profile:
        return Error(f"Unknown flash method '{args.method}'. Use --list to see available methods.")
    
    // 4. Check required external artifacts
    missing_artifacts = []
    for artifact_name in profile.requires:
        artifact = manifest.external.get(artifact_name)
        if not artifact:
            missing_artifacts.append(artifact_name)
            continue
        
        path = external_dir / artifact_name
        if not exists(path):
            if artifact.url:
                print(f"Downloading {artifact_name}...")
                download(artifact.url, path)
                verify_checksum(path, artifact.sha256)
            else:
                missing_artifacts.append(artifact_name)
    
    if missing_artifacts:
        return Error(f"Missing required artifacts: {', '.join(missing_artifacts)}\n"
                    f"Add them with: zigroot external add <name> --type <type> --url <url>")
    
    // 5. Check required tools
    tool = profile.tool
    if tool and not which(tool):
        return Error(f"Required tool '{tool}' not found. Install it and try again.")
    
    // 6. Determine device
    device = args.device or detect_device(profile)
    if not device:
        return Error("No device specified. Use --device <path> or connect a device.")
    
    // 7. Confirmation
    if not args.yes:
        print(f"⚠ WARNING: This will overwrite all data on {device}")
        print(f"  Method: {profile.name}")
        print(f"  Device: {device}")
        confirm = input("Continue? [y/N] ")
        if confirm.lower() != 'y':
            print("Aborted.")
            return Ok()
    
    // 8. Execute flash
    env = {
        "DEVICE": device,
        "IMAGE": "output/rootfs.img",
        "ROOTFS": "build/rootfs",
    }
    for artifact_name in profile.requires:
        env[f"ARTIFACT_{artifact_name.upper()}"] = external_dir / artifact_name
    
    if profile.script:
        result = run_script(profile.script, env=env)
    else:
        // Built-in tool handling
        result = run_flash_tool(profile.tool, device, env)
    
    if result.is_err():
        return Error(f"Flash failed: {result.error}")
    
    print(f"✓ Successfully flashed to {device}")
```

**Errors:**
| Condition | Message | Exit Code |
|-----------|---------|-----------|
| Unknown method | "Unknown flash method 'X'. Use --list to see available methods." | 2 |
| Missing artifact | "Missing required artifacts: bootloader, partition" | 2 |
| Missing tool | "Required tool 'rkflashtool' not found. Install it and try again." | 1 |
| No device | "No device specified. Use --device <path> or connect a device." | 2 |
| Flash failed | "Flash failed: {error}" | 3 |
| Permission denied | "Permission denied. Try running with sudo." | 1 |

---

## Data Models

### Project Directory Structure

```
my-project/
├── .gitignore             # Generated by zigroot init
├── zigroot.toml           # Project manifest
├── zigroot.lock           # Lock file for reproducible builds
├── packages/              # Local package definitions
│   └── myapp/
│       ├── package.toml
│       ├── build.sh       # Optional custom build script
│       ├── install.sh     # Optional custom install script
│       └── patches/
│           └── 001-fix.patch
├── boards/                # Local board definitions (optional)
├── user/
│   ├── files/             # Files to copy to rootfs
│   │   └── etc/
│   │       └── myapp.conf
│   └── scripts/
│       └── post-build.sh  # Post-build hook
├── kernel/                # Kernel configuration (if building kernel)
│   └── .config
├── external/              # Downloaded external artifacts (gitignored)
├── build/                 # Build directory (gitignored)
│   ├── src/               # Extracted sources
│   ├── pkg/               # Per-package build outputs
│   ├── rootfs/            # Assembled rootfs
│   ├── stamps/            # Build stamps for incremental builds
│   └── logs/              # Build logs
├── downloads/             # Downloaded source archives (gitignored)
└── output/                # Final images (gitignored)
    └── rootfs.img
```

### Generated .gitignore

When `zigroot init` runs, it creates (or appends to) a `.gitignore` file:

```gitignore
# zigroot - generated directories
# Do not commit build artifacts, downloads, or generated output

# Build artifacts
build/

# Downloaded source archives
downloads/

# Downloaded external artifacts (bootloader, kernel, etc.)
external/

# Generated output images
output/

# Editor and OS files
*.swp
*.swo
*~
.DS_Store
```

If a `.gitignore` already exists, zigroot appends only the missing entries after a `# zigroot` marker, preserving existing content.

### Example zigroot.toml (generated by `zigroot init`)

When running `zigroot init --board luckfox-pico`, the following `zigroot.toml` is generated:

```toml
[project]
name = "my-embedded-project"
version = "0.1.0"

# Board configuration
# Run 'zigroot board list' to see available boards
# Run 'zigroot board info <name>' for board details
[board]
name = "luckfox-pico"

# Board-specific options (see 'zigroot board info luckfox-pico' for available options)
# [board.options]
# storage = "sd"        # Boot storage: sd, emmc, nor
# uart_console = true   # Enable UART debug console

# Build settings
[build]
# compress = false      # Compress binaries with UPX
# sandbox = false       # Use Docker/Podman for isolated builds
# image_format = "ext4" # Output format: ext4, squashfs, initramfs
# rootfs_size = "64M"   # Root filesystem size

# Packages to include in rootfs
# Run 'zigroot search <query>' to find packages
# Run 'zigroot add <package>' to add packages
[packages]
# busybox = "1.36.1"
# dropbear = "2024.85"

# Package-specific options
# [packages.busybox.options]
# shell = "ash"

# External artifacts (bootloader, kernel, etc.)
# [external.bootloader]
# type = "bootloader"
# url = "https://example.com/u-boot.bin"
# sha256 = "..."

# Remote build cache (optional)
# [cache]
# remote = "s3://my-bucket/zigroot-cache"
```

### Example zigroot.toml (fully configured)

```toml
[project]
name = "my-embedded-project"
version = "1.0.0"

[board]
name = "luckfox-pico"

[board.options]
storage = "emmc"
uart_console = true

[build]
compress = true
sandbox = false
image_format = "ext4"
rootfs_size = "64M"

# Package sources - multiple formats supported:
# - Simple version: "1.36.1" (from default registry)
# - Local path: { path = "packages/myapp" }
# - Git repo: { git = "https://github.com/user/repo", tag = "v1.0.0" }
# - Custom registry: { registry = "https://packages.mycompany.com", version = "1.0.0" }
[packages]
busybox = "1.36.1"                                    # Default registry
dropbear = "2024.85"                                  # Default registry
myapp = { path = "packages/myapp" }                   # Local package
private-lib = { registry = "https://packages.mycompany.com", version = "2.0.0" }
forked-nginx = { git = "https://github.com/myorg/nginx-fork", tag = "v1.24.0-custom" }

[packages.busybox.options]
shell = "ash"

# External artifacts - key is artifact name, type specifies what it is
# Types: bootloader, kernel, partition_table, dtb, firmware, other
[external.u-boot]
type = "bootloader"
url = "https://github.com/luckfox/u-boot/releases/download/v1.0/u-boot.bin"
sha256 = "abc123..."

[external.zImage]
type = "kernel"
path = "external/zImage"  # Local file, no url/sha256 needed

[external.luckfox-partition]
type = "partition_table"
format = "rockchip"       # Partition format: gpt, mbr, rockchip
url = "https://github.com/luckfox/partition/releases/download/v1.0/parameter.txt"
sha256 = "def456..."

[cache]
remote = "s3://my-bucket/zigroot-cache"
```

### Package Source Formats in zigroot.toml

| Format | Example | Description |
|--------|---------|-------------|
| Version string | `"1.36.1"` | Package from default registry |
| Local path | `{ path = "packages/myapp" }` | Local package directory |
| Git with tag | `{ git = "<url>", tag = "v1.0" }` | Git repo at specific tag |
| Git with branch | `{ git = "<url>", branch = "main" }` | Git repo at branch head |
| Git with rev | `{ git = "<url>", rev = "abc123" }` | Git repo at specific commit |
| Custom registry | `{ registry = "<url>", version = "1.0" }` | Package from custom registry |

### Example zigroot.lock

The lock file records exact versions and checksums for reproducible builds. 

#### Package Source Types

The `source` field specifies where a package comes from. It uses a URI-style format:

| Source Type | Format | Description |
|-------------|--------|-------------|
| Registry (default) | `"registry"` or omitted | Official zigroot-packages registry |
| Custom Registry | `"registry:<url>"` | Custom/private package registry |
| Local Path | `"path:<relative_path>"` | Local package in project directory |
| Git Repository | `"git:<url>#<ref>"` | Git repo with tag, branch, or commit |

**Git reference formats:**
- `git:https://github.com/user/repo#v1.0.0` - Tag
- `git:https://github.com/user/repo#main` - Branch
- `git:https://github.com/user/repo#abc1234` - Commit SHA

#### Lock File Example

```toml
# This file is auto-generated by zigroot. Do not edit manually.
# Run 'zigroot update' to update locked versions.

[metadata]
zigroot_version = "0.1.0"
zig_version = "0.13.0"
generated = "2025-01-11T12:00:00Z"

[[package]]
name = "busybox"
version = "1.36.1"
sha256 = "b8cc24c9574d809e7279c3be349795c5d5ceb6fdf19ca709f80cde50e47de314"
# source = "registry" is the default, omitted

[[package]]
name = "dropbear"
version = "2024.85"
sha256 = "def456789..."
depends = ["zlib@1.2.13"]  # Resolved dependencies with exact versions

[[package]]
name = "myapp"
version = "0.1.0"
sha256 = "local"
source = "path:packages/myapp"
depends = ["busybox@1.36.1"]  # Dependencies resolved from package.toml

[[package]]
name = "private-lib"
version = "2.0.0"
sha256 = "abc123..."
source = "registry:https://packages.mycompany.com"

[[package]]
name = "forked-nginx"
version = "1.24.0-custom"
sha256 = "def456..."
source = "git:https://github.com/myorg/nginx-fork#v1.24.0-custom"

[[external]]
name = "u-boot"
type = "bootloader"
sha256 = "abc123..."
url = "https://github.com/luckfox/u-boot/releases/download/v1.0/u-boot.bin"

[[external]]
name = "luckfox-partition"
type = "partition_table"
sha256 = "def456..."
url = "https://github.com/luckfox/partition/releases/download/v1.0/parameter.txt"
```

### Example package.toml (Local Packages)

**Important:** There are two package definition formats:

| Context | Files | Purpose |
|---------|-------|---------|
| **Local packages** (`packages/` dir) | `package.toml` | Single file with all metadata + version info |
| **Registry packages** (`zigroot-packages` repo) | `metadata.toml` + `<version>.toml` | Split to minimize duplication across versions |

Local packages use a single `package.toml` because:
- They're typically single-version (your custom app)
- No need for version-split optimization
- Simpler to maintain

Registry packages split into `metadata.toml` + `<version>.toml` because:
- Adding a new version only requires one small file
- Build instructions defined once, shared across versions
- Reduces duplication in the registry

### Package Source Types

Packages can specify their source code location in multiple ways:

| Source Type | Fields | Use Case |
|-------------|--------|----------|
| URL tarball | `url`, `sha256` | Most common - release tarballs |
| Git tag | `git`, `tag` | Tagged releases from git |
| Git branch | `git`, `branch` | Development/bleeding edge |
| Git commit | `git`, `rev` | Specific commit (most reproducible) |
| Multiple files | `sources[]` | Packages with multiple source archives |

**Source Validation Rules:**

A package MUST specify exactly ONE of these source types:
1. `url` + `sha256` - URL tarball
2. `git` + (`tag` | `branch` | `rev`) - Git repository
3. `sources[]` - Multiple source files

**Error handling:**
- If NONE specified → Error: "Package 'foo' has no source defined. Specify either 'url', 'git', or 'sources'."
- If MULTIPLE specified → Error: "Package 'foo' has conflicting sources. Specify only one of: 'url', 'git', or 'sources'."
- If `git` without ref → Error: "Package 'foo' specifies 'git' but no ref. Add 'tag', 'branch', or 'rev'."
- If `url` without `sha256` → Error: "Package 'foo' specifies 'url' but no 'sha256'. Checksums are required for reproducibility."

**Reproducibility notes:**
- `url` + `sha256` is most reproducible (content-addressed)
- `git` + `tag` is reproducible if tags aren't moved
- `git` + `rev` is reproducible (commit SHA is immutable)
- `git` + `branch` is NOT reproducible - lock file records resolved SHA

#### Local package.toml (single file)

```toml
[package]
name = "openssl"
version = "3.2.0"
description = "TLS/SSL and crypto library"
license = "Apache-2.0"
homepage = "https://www.openssl.org"

[source]
url = "https://www.openssl.org/source/openssl-${version}.tar.gz"
sha256 = "14c826f07c7e433706fb5c69fa9e25dab95684844b4c962a2cf1bf183eb4690e"

[build]
type = "custom"

[options.shared]
type = "bool"
default = false
description = "Build shared libraries"

[options.threads]
type = "bool"
default = true
description = "Enable threading support"
```

#### Local package.toml with git source

For packages that don't provide release tarballs, or for development versions:

```toml
# packages/my-fork/package.toml
[package]
name = "my-fork"
version = "1.0.0-dev"
description = "My custom fork of some-project"
license = "MIT"

[source]
git = "https://github.com/myorg/some-project"
tag = "v1.0.0-custom"
# Or use branch for bleeding edge:
# branch = "main"
# Or use specific commit for reproducibility:
# rev = "abc123def456"

# subdir is useful for monorepos
# subdir = "packages/core"

[build]
type = "cmake"
```

#### Registry metadata.toml (shared across versions)

```toml
# packages/openssl/metadata.toml
[package]
name = "openssl"
description = "TLS/SSL and crypto library"
license = "Apache-2.0"
homepage = "https://www.openssl.org"
keywords = ["tls", "ssl", "crypto", "security"]

[build]
type = "custom"

[options.shared]
type = "bool"
default = false
description = "Build shared libraries"

[options.threads]
type = "bool"
default = true
description = "Enable threading support"
```

#### Registry version file (version-specific)

```toml
# packages/openssl/3.2.0.toml
[release]
version = "3.2.0"
released = "2024-01-30"

[source]
url = "https://www.openssl.org/source/openssl-3.2.0.tar.gz"
sha256 = "14c826f07c7e433706fb5c69fa9e25dab95684844b4c962a2cf1bf183eb4690e"

# Version-specific dependencies (if different from metadata.toml)
# depends = ["zlib>=1.2.11"]
```

### Example package.toml with declarative install rules

For simple local packages, declarative install rules avoid the need for custom scripts:

```toml
# packages/htop/package.toml
[package]
name = "htop"
version = "3.3.0"
description = "Interactive process viewer"
license = "GPL-2.0"
homepage = "https://htop.dev"

[source]
url = "https://github.com/htop-dev/htop/releases/download/${version}/htop-${version}.tar.xz"
sha256 = "..."

[build]
type = "autotools"
configure_args = ["--disable-unicode"]

# Declarative install rules - no install.sh needed
[install]
type = "declarative"
strip = true

[[install.files]]
src = "htop"           # Built binary
dst = "/usr/bin/htop"  # Destination in rootfs
mode = "755"

[[install.files]]
src = "htop.1"
dst = "/usr/share/man/man1/htop.1"
mode = "644"
```

For packages with more complex install needs, use glob patterns:

```toml
[install]
type = "declarative"
strip = true
default_mode = "644"

[[install.files]]
src = "bin/*"
dst = "/usr/bin/"
mode = "755"

[[install.files]]
src = "lib/*.a"
dst = "/usr/lib/"

[[install.files]]
src = "include/**/*.h"
dst = "/usr/include/"
```

### Example kernel package.toml (with GCC toolchain)

Packages that cannot be built with Zig (like the Linux kernel) declare their toolchain needs. This is typically a local package since kernel configs are board-specific:

```toml
# packages/linux-kernel/package.toml
[package]
name = "linux-kernel"
version = "6.6.0"
description = "Linux kernel for embedded systems"
license = "GPL-2.0"

[source]
url = "https://cdn.kernel.org/pub/linux/kernel/v6.x/linux-${version}.tar.xz"
sha256 = "..."

[build]
type = "custom"

# Simple approach: just specify target, Zigroot resolves bootlin URLs automatically
[build.toolchain]
type = "gcc"
target = "arm-linux-gnueabihf"  # Zigroot builds the bootlin URL for each host platform

[options.defconfig]
type = "string"
default = ""
description = "Kernel defconfig to use (e.g., multi_v7_defconfig)"
```

For edge cases where bootlin doesn't have the right toolchain, use explicit URLs:

```toml
[build.toolchain]
type = "gcc-explicit"

[build.toolchain.url]
linux-x86_64 = "https://custom-toolchains.example.com/arm-gcc-linux-x86_64.tar.gz"
linux-aarch64 = "https://custom-toolchains.example.com/arm-gcc-linux-aarch64.tar.gz"
darwin-x86_64 = "https://custom-toolchains.example.com/arm-gcc-darwin-x86_64.tar.gz"
darwin-aarch64 = "https://custom-toolchains.example.com/arm-gcc-darwin-aarch64.tar.gz"
```

### Example board.toml

```toml
[board]
name = "luckfox-pico"
description = "Luckfox Pico (RV1103 SoC, Cortex-A7)"
target = "arm-linux-musleabihf"
cpu = "cortex_a7"
features = ["neon", "vfpv4"]
# kernel = "linux-luckfox"  # Optional: default kernel package for this board
# zigroot_version = ">=0.2.0"  # Optional: minimum zigroot version required

[defaults]
image_format = "ext4"
rootfs_size = "32M"
hostname = "luckfox"

# Required packages for this board (always included)
requires = ["base", "busybox"]

[options.storage]
type = "choice"
default = "sd"
choices = ["sd", "emmc", "nor"]
description = "Boot storage type"

[options.uart_console]
type = "bool"
default = true
description = "Enable UART debug console"

[[flash]]
name = "usb"
description = "Flash via USB using rkflashtool"
tool = "rkflashtool"
requires = ["bootloader", "partition"]  # External artifacts needed
script = "flash-usb.sh"

[[flash]]
name = "sdcard"
description = "Write to SD card"
tool = "dd"
script = "flash-sd.sh"
```

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system—essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

### Property 1: TOML Serialization Round-Trip

*For any* valid Manifest, PackageDefinition, or BoardDefinition, serializing to TOML and then deserializing SHALL produce an equivalent data structure.

**Validates: Requirements 16.3, 17.4, 19.13**

### Property 2: Dependency Build Order

*For any* valid dependency graph without cycles, the computed build order SHALL ensure that every package is built after all of its dependencies.

**Validates: Requirements 20.1, 20.2**

### Property 3: Circular Dependency Detection

*For any* dependency graph containing a cycle, the dependency resolver SHALL detect and report the cycle.

**Validates: Requirements 20.3**

### Property 4: Checksum Verification

*For any* downloaded file with a specified SHA256 checksum, the verification SHALL correctly identify whether the file matches the expected checksum.

**Validates: Requirements 3.2, 8.6, 8.7**

### Property 5: Package Addition Preserves Manifest Validity

*For any* valid Manifest and valid package name, adding the package SHALL produce a valid Manifest that includes the package.

**Validates: Requirements 2.1, 2.2**

### Property 6: Package Removal Preserves Manifest Validity

*For any* valid Manifest containing a package, removing the package SHALL produce a valid Manifest that does not include the package.

**Validates: Requirements 2.5**

### Property 7: Transitive Dependency Inclusion

*For any* package with dependencies, adding that package SHALL also include all transitive dependencies in the resolved package set.

**Validates: Requirements 2.8**

### Property 8: Incremental Build Correctness

*For any* package whose sources and patches have not changed since the last build, the build system SHALL skip rebuilding that package.

**Validates: Requirements 4.4**

### Property 9: Compression Toggle Consistency

*For any* combination of global compress setting and per-package compress setting, the final compression decision SHALL follow the priority: CLI > Package > Global.

**Validates: Requirements 6.1, 6.2, 6.3, 6.4, 6.7, 6.8**

### Property 10: Search Result Grouping

*For any* search query, results SHALL be grouped by type (packages first, then boards) with each result correctly labeled.

**Validates: Requirements 10.2, 10.3, 10.4**

### Property 11: Local Package Priority

*For any* package that exists both locally and in the registry, the local version SHALL be used.

**Validates: Requirements 12.1**

### Property 12: Lock File Reproducibility

*For any* project with a lock file, building with `--locked` SHALL use exactly the versions specified in the lock file.

**Validates: Requirements 13.2, 13.3**

### Property 13: Option Validation

*For any* option with validation constraints (pattern, min, max, allow_empty), invalid values SHALL be rejected with a specific error.

**Validates: Requirements 18.39, 18.40, 18.41, 18.42**

### Property 14: Build Environment Variables

*For any* package build, the environment SHALL contain correctly set variables for CC, TARGET, JOBS, SRCDIR, DESTDIR, PREFIX, and all dependency paths.

**Validates: Requirements 18.20, 18.33**

### Property 15: Minimum Version Enforcement

*For any* package or board with a `zigroot_version` constraint, loading SHALL fail if the current zigroot version does not satisfy the semver constraint.

**Validates: Requirements 30.4, 30.5, 30.6, 30.7**

### Property 16: Semver Compliance

*For any* version comparison, zigroot SHALL follow semver standards where major version changes indicate breaking changes, minor versions add features, and patch versions fix bugs.

**Validates: Requirements 30.8, 31.10**

### Property 17: Source Type Exclusivity

*For any* package definition, specifying more than one source type (url+sha256, git+ref, sources[]) SHALL produce a validation error identifying the conflict.

**Validates: Requirements 18.4, 18.5**

### Property 18: Source Type Requirement

*For any* package definition, specifying no source type SHALL produce a validation error requesting a source.

**Validates: Requirements 18.3, 18.6**

### Property 19: Git Ref Requirement

*For any* package with git source, specifying git without tag, branch, or rev SHALL produce a validation error requesting a ref.

**Validates: Requirements 18.10**

### Property 20: URL Checksum Requirement

*For any* package with url source, specifying url without sha256 SHALL produce a validation error requesting a checksum.

**Validates: Requirements 18.11**

### Property 21: Dependency Conflict Detection

*For any* set of packages with incompatible version constraints, the resolver SHALL detect and report the conflict with a clear error message explaining the conflicting requirements.

**Validates: Requirements 2.9**

### Property 22: Lock File Git SHA Recording

*For any* package with git+branch source, the lock file SHALL record the resolved commit SHA for reproducibility.

**Validates: Requirements 13.7, 18.13**

### Property 23: Board Compatibility Validation

*For any* board and package set, the CLI SHALL validate that all packages are compatible with the board's architecture before building.

**Validates: Requirements 9.3**

### Property 24: Gitignore Append Idempotence

*For any* existing .gitignore file, appending zigroot entries multiple times SHALL produce the same result as appending once (entries are not duplicated).

**Validates: Requirements 1.7**

### Property 25: Download Retry Behavior

*For any* failed download, the system SHALL retry up to 3 times with exponential backoff before reporting failure.

**Validates: Requirements 3.7**

### Property 26: Zig Version Recording

*For any* build, the lock file SHALL record the Zig compiler version used, and a warning SHALL be displayed if the current version differs.

**Validates: Requirements 13.4, 13.5**

### Property 27: Missing Field Error Specificity

*For any* package or board definition missing required fields, the parser SHALL report a specific error identifying the missing field.

**Validates: Requirements 17.5, 28.3, 28.4**

### Property 28: Check Command Validation

*For any* project, running `zigroot check` SHALL validate configuration, check all dependencies, verify toolchains are available, and report what would be built without actually building.

**Validates: Requirements 4.13**

### Property 29: Search Suggestions on Empty Results

*For any* search query that returns no results, the CLI SHALL suggest similar terms or popular items.

**Validates: Requirements 10.9**

### Property 30: Flash Confirmation Requirement

*For any* flash operation without the --yes flag, the CLI SHALL require explicit user confirmation before proceeding.

**Validates: Requirements 7.8**

### Property 31: SDK Completeness

*For any* generated SDK, the tarball SHALL contain the Zig toolchain, all built libraries and headers from packages with `depends` relationships, and a setup script that configures environment variables.

**Validates: Requirements 21.2, 21.3, 21.4**

### Property 32: License Detection Accuracy

*For any* package with a copyleft license (GPL, LGPL), the license report SHALL flag it, and for any package missing license information, the CLI SHALL emit a warning.

**Validates: Requirements 22.4, 22.5**

### Property 33: Dependency Tree Correctness

*For any* package dependency tree, the tree view SHALL correctly distinguish between `depends` (build-time) and `requires` (runtime) relationships.

**Validates: Requirements 23.4**

### Property 34: Cache Key Determinism

*For any* package build, the cache key SHALL be deterministically computed from package version, sha256, target triple, and compiler version, ensuring identical inputs produce identical cache keys.

**Validates: Requirements 24.1, 24.6**

### Property 35: TUI Dependency Auto-Selection

*For any* package selected in the TUI, all of its required dependencies SHALL be automatically selected, and when a package is deselected, a warning SHALL be shown if other selected packages depend on it.

**Validates: Requirements 25.5, 25.6**

## Error Handling

### Error Types

```rust
#[derive(Error, Debug)]
pub enum ZigrootError {
    #[error("Manifest error: {0}")]
    Manifest(#[from] ManifestError),
    
    #[error("Package error: {0}")]
    Package(#[from] PackageError),
    
    #[error("Build error: {0}")]
    Build(#[from] BuildError),
    
    #[error("Download error: {0}")]
    Download(#[from] DownloadError),
    
    #[error("Registry error: {0}")]
    Registry(#[from] RegistryError),
    
    #[error("Zigroot version {current} does not satisfy requirement {required}")]
    VersionMismatch { 
        required: String, 
        current: String,
        suggestion: String,
    },
}

#[derive(Error, Debug)]
pub enum ManifestError {
    #[error("File not found: {path}")]
    NotFound { path: PathBuf },
    
    #[error("Parse error at line {line}: {message}")]
    Parse { line: usize, message: String },
    
    #[error("Missing required field: {field}")]
    MissingField { field: String },
    
    #[error("Invalid value for {field}: {message}")]
    InvalidValue { field: String, message: String },
}

#[derive(Error, Debug)]
pub enum BuildError {
    #[error("Build failed for package {package}: {message}")]
    Failed { package: String, message: String },
    
    #[error("Circular dependency detected: {cycle:?}")]
    CircularDependency { cycle: Vec<String> },
    
    #[error("Missing dependency: {package} requires {dependency}")]
    MissingDependency { package: String, dependency: String },
}
```

### Error Display

Errors are displayed with:
- Clear problem identification
- Suggested solutions when possible
- Colored output (red for errors, yellow for warnings)
- File/line references where applicable

```
✗ Build failed for package 'openssl'

  Error: Configure script failed with exit code 1
  
  Log: build/logs/openssl.log
  
  Suggestion: Check that all dependencies are installed.
              Run 'zigroot doctor' to verify your environment.
```

## Testing Strategy

Zigroot follows strict Test-Driven Development (TDD). All code is written test-first using the red-green-refactor cycle.

### TDD Workflow

1. **Red**: Write a failing test that defines the expected behavior
2. **Green**: Write the minimum code to make the test pass
3. **Refactor**: Improve the code while keeping tests green

```
┌─────────────────────────────────────────────────────────────┐
│                    TDD Cycle                                │
│                                                             │
│    ┌─────────┐      ┌─────────┐      ┌──────────┐          │
│    │  RED    │ ───► │  GREEN  │ ───► │ REFACTOR │ ──┐      │
│    │ (test)  │      │ (code)  │      │ (clean)  │   │      │
│    └─────────┘      └─────────┘      └──────────┘   │      │
│         ▲                                           │      │
│         └───────────────────────────────────────────┘      │
└─────────────────────────────────────────────────────────────┘
```

**Rules:**
- Never write production code without a failing test
- Write only enough test to fail (compilation failures count)
- Write only enough code to pass the failing test
- Refactor only when tests are green

### Test Organization

```
src/
├── core/
│   ├── manifest.rs
│   ├── manifest_tests.rs      # Unit tests (same directory)
│   ├── package.rs
│   ├── package_tests.rs
│   └── ...
├── lib.rs
└── main.rs

tests/                          # Integration tests
├── cli_init_test.rs
├── cli_build_test.rs
├── cli_add_test.rs
└── fixtures/                   # Test data
    ├── valid_manifest.toml
    ├── invalid_manifest.toml
    ├── sample_package/
    └── sample_board/
```

**Naming conventions:**
- Unit test files: `<module>_tests.rs` in same directory
- Integration test files: `cli_<command>_test.rs` in `tests/`
- Test functions: `test_<behavior>_<scenario>` (e.g., `test_parse_manifest_with_missing_board`)
- Property tests: `prop_<property_name>` (e.g., `prop_manifest_roundtrip`)

### Test Coverage Requirements

| Module | Minimum Coverage | Notes |
|--------|------------------|-------|
| `core/` | 80% | Business logic - highest priority |
| `registry/` | 80% | Registry client and caching |
| `infra/` | 70% | I/O operations - harder to test |
| `cli/` | 60% | Thin layer, mostly integration tested |
| **Overall** | **80%** | CI fails below this |

**Coverage tooling:**
```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Run coverage
cargo tarpaulin --out Html --output-dir coverage/

# CI command (fails below threshold)
cargo tarpaulin --fail-under 80
```

### Mock Traits for Isolation

External dependencies are abstracted behind traits for testability:

```rust
// Trait for HTTP operations
pub trait HttpClient: Send + Sync {
    async fn get(&self, url: &str) -> Result<Response>;
    async fn download(&self, url: &str, dest: &Path) -> Result<()>;
}

// Trait for filesystem operations
pub trait FileSystem: Send + Sync {
    fn read_to_string(&self, path: &Path) -> Result<String>;
    fn write(&self, path: &Path, content: &str) -> Result<()>;
    fn exists(&self, path: &Path) -> bool;
    fn create_dir_all(&self, path: &Path) -> Result<()>;
}

// Trait for process execution
pub trait ProcessRunner: Send + Sync {
    fn run(&self, cmd: &str, args: &[&str], env: &HashMap<String, String>) -> Result<Output>;
}

// Mock implementations for testing
#[cfg(test)]
pub struct MockHttpClient {
    responses: HashMap<String, Result<Response>>,
}

#[cfg(test)]
pub struct MockFileSystem {
    files: RefCell<HashMap<PathBuf, String>>,
}

#[cfg(test)]
pub struct MockProcessRunner {
    outputs: HashMap<String, Output>,
}
```

**Usage in tests:**
```rust
#[test]
fn test_fetch_package_verifies_checksum() {
    let mut mock_http = MockHttpClient::new();
    mock_http.expect_download("https://example.com/pkg.tar.gz")
        .returning(|_, dest| {
            std::fs::write(dest, b"fake content")?;
            Ok(())
        });
    
    let fetcher = PackageFetcher::new(Box::new(mock_http));
    let result = fetcher.fetch("pkg", "wrong_checksum");
    
    assert!(matches!(result, Err(PackageError::ChecksumMismatch { .. })));
}
```

### Unit Tests

Unit tests verify individual components in isolation:
- Manifest parsing and serialization
- Package definition parsing
- Dependency resolution algorithms
- Version constraint matching
- Option validation
- Error message formatting

**Example unit test structure:**
```rust
// src/core/manifest_tests.rs

#[cfg(test)]
mod tests {
    use super::*;
    
    mod parse {
        use super::*;
        
        #[test]
        fn test_parse_minimal_manifest() {
            let toml = r#"
                [project]
                name = "test"
                version = "1.0.0"
                
                [board]
                name = "test-board"
            "#;
            
            let manifest = Manifest::from_toml(toml).unwrap();
            assert_eq!(manifest.project.name, "test");
        }
        
        #[test]
        fn test_parse_manifest_missing_project_name() {
            let toml = r#"
                [project]
                version = "1.0.0"
                
                [board]
                name = "test-board"
            "#;
            
            let err = Manifest::from_toml(toml).unwrap_err();
            assert!(err.to_string().contains("missing field `name`"));
        }
    }
    
    mod validate {
        use super::*;
        
        #[test]
        fn test_validate_catches_invalid_board() {
            let manifest = Manifest {
                board: BoardConfig { name: "".to_string(), .. },
                ..Default::default()
            };
            
            let errors = manifest.validate();
            assert!(errors.iter().any(|e| e.field == "board.name"));
        }
    }
}
```

### Property-Based Tests

Property-based tests verify universal properties using the `proptest` crate:

```rust
use proptest::prelude::*;

proptest! {
    /// Feature: zigroot-cli, Property 1: TOML Serialization Round-Trip
    /// Validates: Requirements 16.3, 17.4, 19.13
    #[test]
    fn manifest_roundtrip(manifest in arb_manifest()) {
        let toml = manifest.to_toml().unwrap();
        let parsed = Manifest::from_toml(&toml).unwrap();
        assert_eq!(manifest, parsed);
    }
    
    /// Feature: zigroot-cli, Property 2: Dependency Build Order
    /// Validates: Requirements 20.1, 20.2
    #[test]
    fn build_order_respects_dependencies(graph in arb_dag()) {
        let resolver = DependencyResolver::new(graph.packages);
        let order = resolver.resolve_build_order(&graph.roots).unwrap();
        
        for (i, pkg) in order.iter().enumerate() {
            let deps = graph.dependencies_of(pkg);
            for dep in deps {
                let dep_idx = order.iter().position(|p| p == dep).unwrap();
                assert!(dep_idx < i, "Dependency {} must come before {}", dep, pkg);
            }
        }
    }
    
    /// Feature: zigroot-cli, Property 3: Circular Dependency Detection
    /// Validates: Requirements 20.3
    #[test]
    fn detects_circular_dependencies(graph in arb_cyclic_graph()) {
        let resolver = DependencyResolver::new(graph.packages);
        let cycle = resolver.detect_cycles();
        assert!(cycle.is_some(), "Should detect cycle in cyclic graph");
    }
    
    /// Feature: zigroot-cli, Property 4: Checksum Verification
    /// Validates: Requirements 3.2, 8.6, 8.7
    #[test]
    fn checksum_verification(data: Vec<u8>, expected_hash in arb_sha256()) {
        let actual_hash = sha256(&data);
        let result = verify_checksum(&data, &expected_hash);
        assert_eq!(result.is_ok(), actual_hash == expected_hash);
    }
}
```

### Integration Tests

Integration tests verify end-to-end workflows using real filesystem operations in temporary directories:

```rust
// tests/cli_init_test.rs

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn test_init_creates_project_structure() {
    let temp = TempDir::new().unwrap();
    
    Command::cargo_bin("zigroot")
        .unwrap()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Created zigroot.toml"));
    
    assert!(temp.path().join("zigroot.toml").exists());
    assert!(temp.path().join("packages").is_dir());
    assert!(temp.path().join("boards").is_dir());
    assert!(temp.path().join("user/files").is_dir());
    assert!(temp.path().join("user/scripts").is_dir());
}

#[test]
fn test_init_refuses_nonempty_directory() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("existing.txt"), "content").unwrap();
    
    Command::cargo_bin("zigroot")
        .unwrap()
        .args(["init"])
        .current_dir(temp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("directory is not empty"));
}

#[test]
fn test_init_with_force_in_nonempty_directory() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("existing.txt"), "content").unwrap();
    
    Command::cargo_bin("zigroot")
        .unwrap()
        .args(["init", "--force"])
        .current_dir(temp.path())
        .assert()
        .success();
    
    // Original file preserved
    assert!(temp.path().join("existing.txt").exists());
    // New structure created
    assert!(temp.path().join("zigroot.toml").exists());
}
```

**Integration test fixtures:**
```
tests/fixtures/
├── manifests/
│   ├── minimal.toml           # Bare minimum valid manifest
│   ├── full.toml              # All fields populated
│   ├── with_packages.toml     # Manifest with package references
│   └── invalid/
│       ├── missing_board.toml
│       ├── bad_syntax.toml
│       └── unknown_field.toml
├── packages/
│   ├── simple/                # Minimal package
│   │   └── package.toml
│   ├── with_options/          # Package with options
│   │   └── package.toml
│   └── with_deps/             # Package with dependencies
│       └── package.toml
├── boards/
│   ├── minimal/
│   │   └── board.toml
│   └── full/
│       └── board.toml
└── registry_mock/             # Mock registry structure
    ├── index.json
    └── packages/
        └── busybox/
            ├── metadata.toml
            └── 1.36.1.toml
```

### Golden File Tests

For complex output (error messages, formatted output), use golden file testing:

```rust
#[test]
fn test_error_message_format() {
    let error = PackageError::ChecksumMismatch {
        file: "busybox-1.36.1.tar.gz".to_string(),
        expected: "abc123".to_string(),
        actual: "def456".to_string(),
    };
    
    let output = format_error(&error);
    
    // Compare against golden file
    insta::assert_snapshot!(output);
}
```

Use `insta` crate for snapshot testing:
```bash
cargo install cargo-insta

# Run tests and review snapshots
cargo insta test
cargo insta review
```

### Test Configuration

| Setting | Value | Rationale |
|---------|-------|-----------|
| Property test iterations | 100 minimum | Balance coverage vs speed |
| Integration test timeout | 30 seconds | Prevent hanging tests |
| Coverage threshold | 80% overall | Enforced in CI |
| Parallel test execution | Yes | `cargo test` default |

**Cargo.toml test configuration:**
```toml
[dev-dependencies]
proptest = "1.4"
assert_cmd = "2.0"
predicates = "3.0"
tempfile = "3.10"
insta = "1.34"
mockall = "0.12"

[profile.test]
opt-level = 1  # Faster property tests
```

**CI test command:**
```yaml
- name: Run tests
  run: |
    cargo test --all-features
    cargo tarpaulin --fail-under 80 --out Xml
```

### Test-First Development Checklist

Before implementing any feature:

- [ ] Write unit tests for the happy path
- [ ] Write unit tests for error conditions
- [ ] Write property tests for invariants (if applicable)
- [ ] Write integration test for CLI behavior (if applicable)
- [ ] Verify all tests fail (red)
- [ ] Implement minimum code to pass
- [ ] Refactor while keeping tests green
- [ ] Verify coverage meets threshold

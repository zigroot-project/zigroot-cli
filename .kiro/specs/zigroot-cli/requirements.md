# Requirements Document

## Introduction

Zigroot is a modern embedded Linux rootfs builder designed to be the "npm for embedded systems." It leverages the Zig compiler's built-in cross-compilation capabilities with musl libc to provide a dramatically simpler alternative to Yocto and Buildroot. The tool is implemented as a Rust CLI application that orchestrates package downloads, builds, and image creation with an emphasis on developer experience, reliability, and speed.

The project addresses key pain points in the embedded Linux ecosystem:
- Steep learning curves (Yocto takes weeks to master)
- Complex toolchain setup and cross-compilation nightmares
- Poor incremental build support
- No modern package discovery/sharing mechanisms
- Inability to build from macOS without Docker/VMs

## Core Principles

### Static Linking by Default

Zigroot builds all packages as statically linked binaries using musl libc whenever possible. This design choice provides several benefits:

1. **No dependency hell** - Each binary is self-contained, eliminating version conflicts between shared libraries
2. **Smaller rootfs** - Only include what's actually used, no unused library code
3. **Simpler deployment** - Binaries work on any Linux system with the same architecture
4. **Easier cross-compilation** - No need to manage sysroots with matching library versions
5. **Predictable behavior** - Binary behavior doesn't change based on system library versions

Packages that cannot be statically linked (e.g., those requiring dlopen) are explicitly marked and handled as exceptions.

## Glossary

- **Zigroot_CLI**: The main Rust command-line application that orchestrates all build operations
- **Package**: A software component with build instructions, metadata, and optional patches stored in a standardized format
- **Board**: A hardware target definition containing architecture, CPU features, and board-specific configurations
- **Package_Registry**: The GitHub-hosted repository (zigroot-project/zigroot-packages) containing community packages
- **Board_Registry**: The GitHub-hosted repository (zigroot-project/zigroot-boards) containing board definitions
- **Project**: A user's zigroot workspace created by `zigroot init`, containing configuration and customizations
- **Rootfs**: The root filesystem image containing all compiled packages and system files
- **Target_Triple**: The Zig cross-compilation target specification (e.g., `arm-linux-musleabihf`)
- **Manifest**: The `zigroot.toml` configuration file in a project root defining packages, board, and settings
- **Lock_File**: The `zigroot.lock` file recording exact versions and checksums for reproducible builds
- **Build_Cache**: Local cache of downloaded sources, compiled artifacts, and intermediate build products

## Requirements

### Requirement 1: Project Initialization

**User Story:** As an embedded developer, I want to quickly initialize a new zigroot project, so that I can start building a custom rootfs without manual setup.

#### Acceptance Criteria

1. WHEN a user runs `zigroot init` in an empty directory, THE Zigroot_CLI SHALL create a project structure with `zigroot.toml`, `.gitignore`, `packages/`, `boards/`, `user/files/`, and `user/scripts/` directories
2. WHEN a user runs `zigroot init --board <board_name>`, THE Zigroot_CLI SHALL fetch the board definition from the Board_Registry and configure the project for that target
3. WHEN a user runs `zigroot init` in a non-empty directory without `--force`, THE Zigroot_CLI SHALL display an error and refuse to overwrite existing files
4. WHEN a user runs `zigroot init --force` in a non-empty directory, THE Zigroot_CLI SHALL proceed with initialization, preserving unrelated files
5. THE Zigroot_CLI SHALL generate a default `zigroot.toml` with commented examples for all configuration options
6. THE Zigroot_CLI SHALL generate a `.gitignore` file excluding `build/`, `downloads/`, `output/`, and `external/` directories
7. WHEN a `.gitignore` already exists, THE Zigroot_CLI SHALL append zigroot entries if not already present (with a `# zigroot` comment marker)

### Requirement 2: Package Management

**User Story:** As an embedded developer, I want to add, remove, and update packages easily, so that I can customize my rootfs without manually editing configuration files.

#### Acceptance Criteria

1. WHEN a user runs `zigroot add <package_name>`, THE Zigroot_CLI SHALL fetch the package from the Package_Registry and add it to the Manifest
2. WHEN a user runs `zigroot add <package_name>@<version>`, THE Zigroot_CLI SHALL fetch the specific version and pin it in the Lock_File
3. WHEN a user runs `zigroot add --git <url>#<ref>`, THE Zigroot_CLI SHALL add a package from a git repository
4. WHEN a user runs `zigroot add --registry <url> <package_name>`, THE Zigroot_CLI SHALL add a package from a custom registry
5. WHEN a user runs `zigroot remove <package_name>`, THE Zigroot_CLI SHALL remove the package from the Manifest and update the Lock_File
6. WHEN a user runs `zigroot update`, THE Zigroot_CLI SHALL check for newer versions of all packages and update the Lock_File
7. WHEN a user runs `zigroot update <package_name>`, THE Zigroot_CLI SHALL update only the specified package
8. WHEN a package has dependencies, THE Zigroot_CLI SHALL automatically resolve and add all transitive dependencies
9. IF a dependency conflict occurs, THEN THE Zigroot_CLI SHALL display a clear error message explaining the conflict and suggest resolution options
10. WHEN a user runs `zigroot package list`, THE Zigroot_CLI SHALL display all installed packages with their versions and descriptions
11. WHEN a user runs `zigroot package info <package_name>`, THE Zigroot_CLI SHALL display detailed package information including version, description, dependencies, and license
12. THE Zigroot_CLI SHALL cache the package index locally and refresh it periodically

### Requirement 3: Package Download

**User Story:** As an embedded developer, I want to download all package sources before building, so that I can work offline and verify downloads.

#### Acceptance Criteria

1. WHEN a user runs `zigroot fetch`, THE Zigroot_CLI SHALL download source archives for all packages in the Manifest
2. WHEN downloading a package, THE Zigroot_CLI SHALL verify the SHA256 checksum against the package definition
3. IF a checksum verification fails, THEN THE Zigroot_CLI SHALL delete the corrupted download and report the error
4. WHEN a source archive already exists with valid checksum, THE Zigroot_CLI SHALL skip the download
5. THE Zigroot_CLI SHALL display download progress with speed and ETA for each package
6. WHEN a user runs `zigroot fetch --parallel <n>`, THE Zigroot_CLI SHALL download up to n packages concurrently
7. IF a download fails, THEN THE Zigroot_CLI SHALL retry up to 3 times with exponential backoff before reporting failure
8. WHEN a user runs `zigroot fetch --force`, THE Zigroot_CLI SHALL re-download all packages even if they already exist with valid checksums

### Requirement 4: Build System

**User Story:** As an embedded developer, I want to build my rootfs with a single command, so that I can iterate quickly on my embedded system.

#### Acceptance Criteria

1. WHEN a user runs `zigroot build`, THE Zigroot_CLI SHALL compile all packages and assemble the rootfs
2. THE Zigroot_CLI SHALL use Zig's cross-compilation with the Target_Triple from the board definition
3. THE Zigroot_CLI SHALL build all packages as statically linked binaries using musl libc by default
4. WHEN a package has already been built and its sources/patches haven't changed, THE Zigroot_CLI SHALL skip rebuilding that package
5. WHEN a user runs `zigroot clean`, THE Zigroot_CLI SHALL remove all build artifacts
6. WHEN a user runs `zigroot build --package <name>`, THE Zigroot_CLI SHALL rebuild only the specified package
7. THE Zigroot_CLI SHALL display build progress with package name, step, and elapsed time
8. IF a build fails, THEN THE Zigroot_CLI SHALL display the error output and identify the failing package clearly
9. WHEN a user runs `zigroot build --jobs <n>`, THE Zigroot_CLI SHALL limit parallel compilation to n jobs
10. THE Zigroot_CLI SHALL support ccache integration when available on the host system
11. WHEN building completes successfully, THE Zigroot_CLI SHALL display a summary with image size and build time
12. THE Zigroot_CLI SHALL set compiler flags for static linking (`-static`) and musl target automatically
13. WHEN a user runs `zigroot check`, THE Zigroot_CLI SHALL validate configuration, check all dependencies, verify toolchains are available, and report what would be built without actually building

### Requirement 5: Image Creation

**User Story:** As an embedded developer, I want to create flashable images in various formats, so that I can deploy to different target devices.

#### Acceptance Criteria

1. WHEN a user runs `zigroot build`, THE Zigroot_CLI SHALL create a `rootfs.img` ext4 image by default
2. WHEN a user specifies `image_format = "squashfs"` in the Manifest, THE Zigroot_CLI SHALL create a SquashFS image
3. WHEN a user specifies `image_format = "initramfs"` in the Manifest, THE Zigroot_CLI SHALL create a cpio initramfs archive
4. THE Zigroot_CLI SHALL set the image size based on the `rootfs_size` configuration in the Manifest
5. WHEN the assembled rootfs exceeds the configured size, THE Zigroot_CLI SHALL report an error with the actual size needed
6. THE Zigroot_CLI SHALL include all user files from `user/files/` in the final image
7. THE Zigroot_CLI SHALL execute `user/scripts/post-build.sh` if present before image creation

### Requirement 6: Binary Compression

**User Story:** As an embedded developer, I want to optionally compress binaries to reduce image size, so that I can fit more functionality on space-constrained devices.

#### Acceptance Criteria

1. WHEN a user specifies `compress = true` in the Manifest `[build]` section, THE Zigroot_CLI SHALL compress all binaries using UPX by default
2. WHEN a user specifies `compress = false` in the Manifest `[build]` section, THE Zigroot_CLI SHALL not compress any binaries by default
3. WHEN a package specifies `compress = true` in its `package.toml`, THE Zigroot_CLI SHALL compress that package regardless of global setting
4. WHEN a package specifies `compress = false` in its `package.toml`, THE Zigroot_CLI SHALL skip compression for that package regardless of global setting
5. THE Zigroot_CLI SHALL only compress binaries for architectures supported by UPX (x86, x86_64, ARM32, ARM64)
6. IF UPX is not installed on the host system, THEN THE Zigroot_CLI SHALL display a warning and skip compression
7. WHEN a user runs `zigroot build --compress`, THE Zigroot_CLI SHALL enable compression for all packages (overrides manifest and package settings)
8. WHEN a user runs `zigroot build --no-compress`, THE Zigroot_CLI SHALL disable compression for all packages (overrides manifest and package settings)
9. THE Zigroot_CLI SHALL display compression statistics (original size, compressed size, ratio) in the build summary
10. WHEN compression fails for a binary, THE Zigroot_CLI SHALL log a warning and continue with the uncompressed binary

### Requirement 7: Device Flashing

**User Story:** As an embedded developer, I want to flash my image directly to a device, so that I can quickly test my builds.

#### Acceptance Criteria

1. WHEN a user runs `zigroot flash`, THE Zigroot_CLI SHALL list available flash methods for the current board
2. WHEN a user runs `zigroot flash <method>`, THE Zigroot_CLI SHALL execute the specified flash method
3. WHEN a board definition includes flash profiles, THE Zigroot_CLI SHALL use the profile's tool and script
4. A flash profile MAY specify required external artifacts (bootloader, kernel, partition table)
5. WHEN a flash profile requires external artifacts, THE Zigroot_CLI SHALL download them if URLs are provided or prompt for local paths
6. THE Zigroot_CLI SHALL cache downloaded external artifacts in the build directory
7. WHEN a user runs `zigroot flash --device <path>`, THE Zigroot_CLI SHALL use the specified device path
8. THE Zigroot_CLI SHALL require explicit confirmation before flashing to prevent accidental data loss
9. WHEN a user runs `zigroot flash --yes`, THE Zigroot_CLI SHALL skip the confirmation prompt
10. IF no flash method is defined for the board, THEN THE Zigroot_CLI SHALL display instructions for manual flashing
11. WHEN a user runs `zigroot flash --list`, THE Zigroot_CLI SHALL display all available flash methods with descriptions
12. THE Zigroot_CLI SHALL validate that required flash tools are installed before attempting to flash

### Requirement 8: External Artifacts

**User Story:** As an embedded developer, I want to include external artifacts like bootloaders, kernels, and partition tables, so that I can create complete flashable images.

#### Acceptance Criteria

1. THE Manifest MAY specify external artifacts in an `[external]` section with name, type, and either url or path
2. THE Zigroot_CLI SHALL support artifact types: `bootloader`, `kernel`, `partition_table`, `dtb`, `firmware`, `other`
3. WHEN an external artifact specifies `url`, THE Zigroot_CLI SHALL download it during `zigroot fetch`
4. WHEN an external artifact specifies `path`, THE Zigroot_CLI SHALL use the local file relative to the project root
5. AN external artifact MAY specify both `url` and `path` - the file is downloaded to the specified path
6. WHEN an external artifact specifies `url`, THE artifact SHALL also specify `sha256` for verification
7. WHEN an artifact has a `sha256` field, THE Zigroot_CLI SHALL verify the checksum after download or when using local file
8. WHEN a flash profile references an external artifact, THE Zigroot_CLI SHALL include it in the flash process
9. WHEN a user runs `zigroot external list`, THE Zigroot_CLI SHALL display all configured external artifacts and their status (downloaded/local/missing)
10. WHEN a user runs `zigroot external add <name> --type <type> --url <url>`, THE Zigroot_CLI SHALL add a remote artifact to the Manifest
11. WHEN a user runs `zigroot external add <name> --type <type> --path <path>`, THE Zigroot_CLI SHALL add a local artifact to the Manifest
12. A partition table artifact MAY specify `format` as `gpt`, `mbr`, or `rockchip` (for vendor-specific formats)
13. WHEN a board requires a partition table, THE flash profile SHALL reference it and THE Zigroot_CLI SHALL apply it during flashing

### Requirement 9: Board Management

**User Story:** As an embedded developer, I want to easily select and configure target boards, so that I can build for different hardware platforms.

#### Acceptance Criteria

1. WHEN a user runs `zigroot board list`, THE Zigroot_CLI SHALL list all available boards from the Board_Registry
2. WHEN a user runs `zigroot board set <board_name>`, THE Zigroot_CLI SHALL update the Manifest with the new board configuration
3. THE Zigroot_CLI SHALL validate that the selected board is compatible with the current packages
4. WHEN a user runs `zigroot board info <board_name>`, THE Zigroot_CLI SHALL display board details including architecture, CPU, and supported features

### Requirement 10: Unified Search

**User Story:** As an embedded developer, I want to search across packages and boards with a single command, so that I can quickly discover available components.

#### Acceptance Criteria

1. WHEN a user runs `zigroot search <query>`, THE Zigroot_CLI SHALL search both Package_Registry and Board_Registry for matching items
2. THE Zigroot_CLI SHALL display search results grouped by type (packages first, then boards) with clear visual separation
3. FOR each package result, THE Zigroot_CLI SHALL display: name, version, description, and a [package] label
4. FOR each board result, THE Zigroot_CLI SHALL display: name, architecture, description, and a [board] label
5. WHEN a user runs `zigroot search --packages <query>`, THE Zigroot_CLI SHALL search only packages
6. WHEN a user runs `zigroot search --boards <query>`, THE Zigroot_CLI SHALL search only boards
7. WHEN a user runs `zigroot search --refresh <query>`, THE Zigroot_CLI SHALL force a refresh of both indexes before searching
8. THE Zigroot_CLI SHALL highlight matching terms in the search results
9. WHEN no results are found, THE Zigroot_CLI SHALL suggest similar terms or popular items

### Requirement 11: Configuration Management

**User Story:** As an embedded developer, I want to configure my build through a simple configuration file, so that I can version control my settings.

#### Acceptance Criteria

1. THE Zigroot_CLI SHALL read configuration from `zigroot.toml` in the project root
2. THE Zigroot_CLI SHALL support environment variable substitution in configuration values using `${VAR}` syntax
3. WHEN a required configuration value is missing, THE Zigroot_CLI SHALL display a helpful error with the expected format
4. THE Zigroot_CLI SHALL validate the Manifest schema and report all errors before starting a build
5. THE Zigroot_CLI SHALL support configuration inheritance through `extends = "<base_config>"` directive

### Requirement 12: Local Package Development

**User Story:** As an embedded developer, I want to create and test custom packages locally, so that I can add proprietary or modified software to my rootfs.

#### Acceptance Criteria

1. WHEN a package exists in the local `packages/` directory, THE Zigroot_CLI SHALL use it instead of fetching from the registry
2. WHEN a user runs `zigroot build --package <local_package>`, THE Zigroot_CLI SHALL build only that package for rapid iteration
3. THE Zigroot_CLI SHALL support `file://` URLs in package definitions for local source archives

### Requirement 13: Reproducible Builds

**User Story:** As an embedded developer, I want reproducible builds, so that I can reliably recreate the same image from the same inputs.

#### Acceptance Criteria

1. THE Zigroot_CLI SHALL generate a `zigroot.lock` file recording exact versions and checksums of all packages
2. WHEN a Lock_File exists, THE Zigroot_CLI SHALL use the locked versions instead of resolving latest versions
3. WHEN a user runs `zigroot build --locked`, THE Zigroot_CLI SHALL fail if any package would differ from the Lock_File
4. THE Zigroot_CLI SHALL record the Zig compiler version in the Lock_File
5. IF the current Zig version differs from the Lock_File, THEN THE Zigroot_CLI SHALL display a warning
6. THE Lock_File SHALL record the package source using URI-style format: `registry` (default, omitted), `registry:<url>`, `path:<relative_path>`, or `git:<url>#<ref>`
7. WHEN a package source is `git:<url>#<ref>`, THE Zigroot_CLI SHALL record the resolved commit SHA in the Lock_File for reproducibility

### Requirement 14: Error Handling and Diagnostics

**User Story:** As an embedded developer, I want clear error messages and diagnostics, so that I can quickly identify and fix problems.

#### Acceptance Criteria

1. WHEN an error occurs, THE Zigroot_CLI SHALL display a clear message identifying the problem and suggesting solutions
2. THE Zigroot_CLI SHALL use colored output to distinguish errors, warnings, and informational messages
3. WHEN a user runs any command with `--verbose`, THE Zigroot_CLI SHALL display detailed diagnostic information
4. WHEN a build fails, THE Zigroot_CLI SHALL preserve build logs in `build/logs/<package>.log`
5. WHEN a user runs `zigroot doctor`, THE Zigroot_CLI SHALL check system dependencies and report any issues
6. THE Zigroot_CLI SHALL detect common misconfigurations and provide specific guidance

### Requirement 15: Interactive Output and Progress

**User Story:** As an embedded developer, I want beautiful, animated output with progress indicators, so that I can see what's happening and estimate completion time.

#### Acceptance Criteria

1. WHEN running in an interactive terminal, THE Zigroot_CLI SHALL display animated spinners for operations with unknown duration
2. WHEN downloading packages, THE Zigroot_CLI SHALL display a progress bar with percentage, download speed, and ETA
3. WHEN building packages, THE Zigroot_CLI SHALL display a progress bar showing completed packages out of total
4. THE Zigroot_CLI SHALL use npm-style colorful output with emojis for visual clarity (✓ success, ✗ error, ⚠ warning)
5. WHEN multiple operations run in parallel, THE Zigroot_CLI SHALL display a multi-line progress view showing each operation's status
6. WHEN running in non-interactive mode (piped output), THE Zigroot_CLI SHALL fall back to simple line-by-line output without animations
7. THE Zigroot_CLI SHALL display a summary banner on completion showing total time, packages built, and image size
8. WHEN a user runs any command with `--quiet`, THE Zigroot_CLI SHALL suppress all output except errors
9. THE Zigroot_CLI SHALL use consistent color coding: green for success, red for errors, yellow for warnings, blue for info, dim for secondary text
10. WHEN a user runs any command with `--json`, THE Zigroot_CLI SHALL output machine-readable JSON format for scripting and CI integration

### Requirement 16: Manifest Serialization

**User Story:** As a developer, I want the manifest to be reliably saved and loaded, so that my configuration is preserved correctly.

#### Acceptance Criteria

1. WHEN the Zigroot_CLI writes the Manifest to disk, THE Zigroot_CLI SHALL serialize it as valid TOML
2. WHEN the Zigroot_CLI reads the Manifest from disk, THE Zigroot_CLI SHALL parse it and produce an equivalent data structure
3. FOR ALL valid Manifest configurations, serializing then deserializing SHALL produce an equivalent Manifest (round-trip property)

### Requirement 17: Package Definition Parsing

**User Story:** As a developer, I want package definitions to be parsed correctly, so that builds use the correct sources and build instructions.

#### Acceptance Criteria

1. WHEN the Zigroot_CLI parses a local package definition (`package.toml`), THE Zigroot_CLI SHALL extract all required fields (name, version, description, url, sha256)
2. WHEN the Zigroot_CLI parses a registry package, THE Zigroot_CLI SHALL merge `metadata.toml` and `<version>.toml` into a complete package definition
3. WHEN a package definition contains optional fields (depends, requires, patches, build), THE Zigroot_CLI SHALL parse them correctly
4. FOR ALL valid Package definitions (both local and registry formats), parsing then serializing SHALL produce an equivalent definition (round-trip property)
5. IF a package definition is missing required fields, THEN THE Zigroot_CLI SHALL report a specific error identifying the missing field

### Requirement 18: Package Definition Format

**User Story:** As a package maintainer, I want a clear and flexible package definition format, so that I can easily create and maintain packages.

#### Acceptance Criteria

1. THE Zigroot_CLI SHALL use TOML format for local package definitions in `package.toml` (single file containing all metadata and version info)
2. THE Zigroot_CLI SHALL use TOML format for registry packages with split files: `metadata.toml` (shared across versions) and `<version>.toml` (version-specific)
3. A local package definition SHALL contain required fields: `name`, `version`, `description`, and exactly ONE source type
4. A package SHALL specify exactly ONE source type: `url`+`sha256`, `git`+ref, or `sources[]`
5. IF a package specifies multiple source types, THEN THE Zigroot_CLI SHALL report an error identifying the conflict
6. IF a package specifies no source type, THEN THE Zigroot_CLI SHALL report an error requesting a source
7. A registry metadata.toml SHALL contain required fields: `name`, `description` and MAY contain: `license`, `homepage`, `keywords`, `build`, `options`
8. A registry version.toml SHALL contain required fields: `version`, and exactly ONE source type, and MAY contain: `released`, `dependencies`
9. A package source MAY specify `git` with `tag`, `branch`, or `rev` as an alternative to `url`+`sha256`
10. IF a package specifies `git` without `tag`, `branch`, or `rev`, THEN THE Zigroot_CLI SHALL report an error requesting a ref
11. IF a package specifies `url` without `sha256`, THEN THE Zigroot_CLI SHALL report an error requesting a checksum
12. WHEN a package specifies `git` source, THE Zigroot_CLI SHALL clone the repository and checkout the specified ref
13. WHEN a package specifies `git` with `branch`, THE Lock_File SHALL record the resolved commit SHA for reproducibility
14. A package definition MAY contain optional fields: `license`, `homepage`, `depends`, `requires`, `patches`, `files`
15. WHEN a package specifies `depends`, THE Zigroot_CLI SHALL build those packages first and make their build outputs (libraries, headers) available for compilation
16. WHEN a package specifies `requires`, THE Zigroot_CLI SHALL ensure those packages are included in the final rootfs
17. A package MAY specify `build.type` to use a predefined build system: `autotools`, `cmake`, `meson`, `make`, `custom`
18. A package MAY specify `build.steps` as an array of build commands for fine-grained control
19. Each build step SHALL have `run` (command) and optional `args` (arguments array)
20. Build steps MAY use variable substitution: `${TARGET}`, `${CC}`, `${JOBS}`, `${SRCDIR}`, `${DESTDIR}`, `${PREFIX}`
21. WHEN `build.type` is `autotools`, THE Zigroot_CLI SHALL run configure/make/make install with standard cross-compilation flags
22. WHEN `build.type` is `cmake`, THE Zigroot_CLI SHALL run cmake with cross-compilation toolchain file
23. WHEN `build.type` is `meson`, THE Zigroot_CLI SHALL run meson with cross-compilation configuration
24. WHEN `build.type` is `make`, THE Zigroot_CLI SHALL run make with CC/AR/etc set to Zig toolchain
25. WHEN `build.type` is `custom` or unspecified, THE Zigroot_CLI SHALL execute `build.sh` script
26. WHEN `build.steps` is specified, THE Zigroot_CLI SHALL execute each step in order, ignoring `build.type`
27. A package MAY specify `build.configure_args`, `build.make_args`, `build.cmake_args` to customize build commands
28. THE Zigroot_CLI SHALL support version constraints in dependencies using semver syntax (e.g., `>=1.0`, `^2.0`, `~1.2`)
29. A package definition MAY specify `arch` to limit compatibility to specific architectures
30. A package definition MAY specify `provides` to declare virtual packages it satisfies
31. A package definition MAY specify `conflicts` to declare incompatible packages
32. THE Zigroot_CLI SHALL support `sources` as an alternative to `url` for packages with multiple source files
33. THE Zigroot_CLI SHALL expose dependency build outputs via environment variables (`$ZIGROOT_PKG_<NAME>_DIR`) during build
34. A package definition MAY contain an `[options]` section defining configurable build options
35. Each option SHALL have a name, type (bool, string, choice, number), default value, and description
36. WHEN an option type is `choice`, THE option SHALL specify a list of valid values
37. THE Zigroot_CLI SHALL pass option values to build scripts via environment variables (`$ZIGROOT_OPT_<NAME>`)
38. THE user MAY override package options in `zigroot.toml` under `[packages.<name>.options]`
39. A string option MAY specify `pattern` as a regex for validation
40. A string option MAY specify `allow_empty = false` to require a non-empty value
41. WHEN an option value fails validation, THE Zigroot_CLI SHALL report the error with the expected format
42. A numeric option MAY specify `min` and `max` bounds for validation
43. A package MAY specify an `install.sh` script for custom installation steps after build
44. A package MAY specify declarative `[[install.files]]` rules as an alternative to `install.sh`
45. WHEN no `install.sh` exists and no `[[install.files]]` rules exist, THE Zigroot_CLI SHALL use default installation based on build type

### Requirement 19: Board Definition Format

**User Story:** As a board maintainer, I want a clear board definition format, so that I can easily add support for new hardware.

#### Acceptance Criteria

1. THE Zigroot_CLI SHALL use TOML format for board definitions in `board.toml`
2. A board definition SHALL contain a `[board]` section with required fields: `name`, `description`, `target` (Zig target triple), `cpu`
3. The `[board]` section MAY contain optional fields: `features`, `kernel`, `zigroot_version`
4. A board definition SHALL contain a `[defaults]` section with: `image_format`, `rootfs_size`, `hostname`
5. WHEN a board requires specific packages, THE board definition SHALL specify them in `requires` array at the top level
6. A board definition MAY contain one or more `[[flash]]` sections defining flash methods
7. Each flash section SHALL contain: `name`, `description`, and either `script` or `tool`
8. Each flash section MAY specify `requires` listing external artifacts needed for flashing
9. A board definition MAY contain `[options.<name>]` sections defining configurable board options
10. Board options SHALL follow the same format as package options (type, default, description, validation)
11. THE Zigroot_CLI SHALL pass board option values to build/flash scripts via environment variables (`$ZIGROOT_BOARD_OPT_<NAME>`)
12. THE user MAY override board options in `zigroot.toml` under `[board.options]`
13. FOR ALL valid Board definitions, parsing then serializing SHALL produce an equivalent definition (round-trip property)

### Requirement 20: Dependency Resolution

**User Story:** As an embedded developer, I want dependencies to be resolved correctly, so that all required packages are included in my build.

#### Acceptance Criteria

1. WHEN resolving dependencies, THE Zigroot_CLI SHALL compute a valid topological order for building packages
2. FOR ALL valid dependency graphs, THE Zigroot_CLI SHALL produce a build order where each package is built after its dependencies
3. IF a circular dependency exists, THEN THE Zigroot_CLI SHALL detect it and report the cycle
4. WHEN multiple versions of a package are requested, THE Zigroot_CLI SHALL select a compatible version satisfying all constraints or report a conflict if no compatible version exists


### Requirement 21: SDK Generation

**User Story:** As an embedded developer, I want to generate a standalone SDK, so that I can develop and cross-compile applications outside of zigroot.

#### Acceptance Criteria

1. WHEN a user runs `zigroot sdk`, THE Zigroot_CLI SHALL generate a standalone SDK tarball
2. THE SDK SHALL contain the Zig toolchain configured for the target architecture
3. THE SDK SHALL contain all built libraries and headers from packages with `depends` relationships
4. THE SDK SHALL include a setup script that configures environment variables (CC, CFLAGS, etc.)
5. WHEN a user runs `zigroot sdk --output <path>`, THE Zigroot_CLI SHALL save the SDK to the specified path
6. THE SDK SHALL be usable without zigroot installed on the development machine

### Requirement 22: License Compliance

**User Story:** As a product developer, I want to track licenses of all included packages, so that I can ensure legal compliance.

#### Acceptance Criteria

1. WHEN a user runs `zigroot license`, THE Zigroot_CLI SHALL display a summary of all package licenses
2. WHEN a user runs `zigroot license --export <path>`, THE Zigroot_CLI SHALL generate a license report file
3. THE license report SHALL include: package name, version, license type, license text, and source URL
4. WHEN a package has a copyleft license (GPL, LGPL), THE Zigroot_CLI SHALL flag it in the report
5. THE Zigroot_CLI SHALL warn if any package is missing license information
6. WHEN a user runs `zigroot license --sbom`, THE Zigroot_CLI SHALL generate an SPDX-compatible Software Bill of Materials

### Requirement 23: Dependency Visualization

**User Story:** As an embedded developer, I want to visualize package dependencies, so that I can understand the build structure.

#### Acceptance Criteria

1. WHEN a user runs `zigroot tree`, THE Zigroot_CLI SHALL display a tree view of package dependencies
2. WHEN a user runs `zigroot tree --graph`, THE Zigroot_CLI SHALL generate a DOT format graph file
3. WHEN a user runs `zigroot tree <package>`, THE Zigroot_CLI SHALL show dependencies for that specific package
4. THE dependency view SHALL distinguish between `depends` (build-time) and `requires` (runtime) relationships
5. THE Zigroot_CLI SHALL detect and highlight circular dependencies

### Requirement 24: Build Cache Sharing

**User Story:** As a team lead, I want to share build caches between developers and CI, so that we can reduce build times.

#### Acceptance Criteria

1. THE Zigroot_CLI SHALL store build artifacts in a content-addressable cache
2. WHEN a user runs `zigroot cache export <path>`, THE Zigroot_CLI SHALL export the cache to a tarball
3. WHEN a user runs `zigroot cache import <path>`, THE Zigroot_CLI SHALL import a cache tarball
4. THE Zigroot_CLI MAY support remote cache via `[cache] remote = "s3://bucket/path"` in the Manifest
5. WHEN a remote cache is configured, THE Zigroot_CLI SHALL check for cached artifacts before building
6. THE cache key SHALL include: package version, sha256, target triple, and compiler version
7. WHEN a user runs `zigroot cache clean`, THE Zigroot_CLI SHALL clear the cache directory
8. WHEN a user runs `zigroot cache info`, THE Zigroot_CLI SHALL display cache size and location


### Requirement 25: Interactive Configuration (TUI)

**User Story:** As an embedded developer, I want a menuconfig-style TUI for configuration, so that I can easily browse and select packages and options.

#### Acceptance Criteria

1. WHEN a user runs `zigroot config`, THE Zigroot_CLI SHALL launch an interactive TUI configuration interface
2. THE TUI SHALL display a hierarchical menu with categories: Board, Packages, Build Options, External Artifacts
3. THE TUI SHALL allow browsing available packages from the registry with descriptions and search
4. THE TUI SHALL allow selecting/deselecting packages with space bar and show dependencies
5. WHEN a package is selected, THE TUI SHALL automatically select its required dependencies
6. WHEN a package is deselected, THE TUI SHALL warn if other packages depend on it
7. THE TUI SHALL display package details (version, license, size estimate) in a side panel
8. THE TUI SHALL allow configuring build options: compression, image format, rootfs size
9. WHEN the user saves and exits, THE Zigroot_CLI SHALL update `zigroot.toml` with the new configuration
10. THE TUI SHALL support keyboard navigation (arrows, enter, escape, tab) and vim-style keys (j/k/h/l)
11. WHEN a user runs `zigroot config --board`, THE Zigroot_CLI SHALL show only board selection
12. WHEN a user runs `zigroot config --packages`, THE Zigroot_CLI SHALL show only package selection
13. THE TUI SHALL highlight packages already in the current configuration
14. THE TUI SHALL show a diff of changes before saving
15. WHEN a selected package has configurable options, THE TUI SHALL allow drilling into a submenu to configure them
16. THE TUI SHALL display option types (bool as checkbox, choice as dropdown, string as text input)
17. THE TUI SHALL show option descriptions and default values


### Requirement 26: Kernel Building

**User Story:** As an embedded developer, I want to build Linux kernels for my target board, so that I can customize kernel configuration and modules.

#### Acceptance Criteria

1. THE Zigroot_CLI SHALL support kernel packages that are built using GCC instead of Zig
2. A kernel package SHALL specify `[build.toolchain]` with `type = "gcc"` and `target` to indicate GCC-based build
3. WHEN `type = "gcc"` is specified, THE Zigroot_CLI SHALL automatically resolve the bootlin.com toolchain URL for the current host platform
4. THE Zigroot_CLI SHALL support common target triples: `arm-linux-gnueabihf`, `aarch64-linux-gnu`, `x86_64-linux-gnu`, `riscv64-linux-gnu`
5. A package MAY specify `libc` (default: "glibc") and `release` (default: "stable-2024.02-1") to customize the bootlin toolchain
6. A package MAY use `type = "gcc-explicit"` with `[build.toolchain.url]` to provide custom URLs per host platform
7. THE Zigroot_CLI SHALL cache downloaded GCC toolchains for reuse across builds
8. IF bootlin toolchains are not available for the current host (e.g., macOS), THEN THE Zigroot_CLI SHALL suggest using explicit URLs or Docker
9. A kernel package MAY specify `defconfig` to use a predefined kernel configuration
10. A kernel package MAY specify `config_fragments` to apply configuration fragments on top of defconfig
11. WHEN a user runs `zigroot kernel menuconfig`, THE Zigroot_CLI SHALL launch the kernel's menuconfig interface
12. THE Zigroot_CLI SHALL save kernel configuration changes to the project's `kernel/` directory
13. A board definition MAY specify `kernel` to reference a kernel package and configuration
14. WHEN building a kernel, THE Zigroot_CLI SHALL also build specified kernel modules
15. THE Zigroot_CLI SHALL install kernel modules to the rootfs under `/lib/modules/<version>/`
16. WHEN a user runs `zigroot build --kernel-only`, THE Zigroot_CLI SHALL build only the kernel and modules


### Requirement 27: Build Isolation

**User Story:** As a security-conscious developer, I want builds to be isolated, so that malicious packages cannot harm my host system.

#### Acceptance Criteria

1. THE Zigroot_CLI SHALL run package builds without isolation by default for maximum compatibility
2. WHEN a user runs `zigroot build --sandbox`, THE Zigroot_CLI SHALL use Docker/Podman for container isolation
3. THE Manifest MAY specify `[build] sandbox = true` to enable container isolation by default
4. WHEN container isolation is enabled, THE Zigroot_CLI SHALL run builds inside a minimal container with only build tools
5. WHEN container isolation is enabled but Docker/Podman is not available, THE Zigroot_CLI SHALL display an error and refuse to build
6. THE container SHALL have read access to source directory and write access to build/output directories
7. THE container SHALL block network access during build by default
8. WHEN a package specifies `build.network = true`, THE Zigroot_CLI SHALL allow network access for that package (with warning)
9. WHEN a user runs `zigroot build --no-sandbox`, THE Zigroot_CLI SHALL disable isolation regardless of manifest setting


### Requirement 28: Package Authoring

**User Story:** As a package maintainer, I want tools to create, validate, and publish packages, so that I can contribute to the zigroot ecosystem.

#### Acceptance Criteria

1. WHEN a user runs `zigroot package new <name>`, THE Zigroot_CLI SHALL create a package template in `packages/<name>/` with `metadata.toml` and a version file
2. WHEN a user runs `zigroot verify <path>`, THE Zigroot_CLI SHALL validate the package or board structure, required fields, and TOML syntax
3. THE validation SHALL check that `metadata.toml` contains required fields: name, description, license
4. THE validation SHALL check that version files contain required fields: version, source.url, source.sha256
5. WHEN a user runs `zigroot verify --fetch <path>`, THE Zigroot_CLI SHALL download the source and verify the SHA256 checksum
6. WHEN a user runs `zigroot package test <path>`, THE Zigroot_CLI SHALL attempt to build the package and report success or failure
7. WHEN a user runs `zigroot publish <path>`, THE Zigroot_CLI SHALL create a PR to the appropriate registry (zigroot-packages or zigroot-boards)
8. THE publish command SHALL validate the package/board before creating the PR
9. THE publish command SHALL require GitHub authentication via `GITHUB_TOKEN` environment variable or `gh` CLI
10. WHEN publishing, THE Zigroot_CLI SHALL check that the package name doesn't conflict with existing packages (unless updating)
11. WHEN publishing a new version of an existing package, THE Zigroot_CLI SHALL only add the new version file
12. THE Zigroot_CLI SHALL support `zigroot package bump <path> <version>` to create a new version file from the latest


### Requirement 29: Board Authoring

**User Story:** As a board maintainer, I want tools to create, validate, and publish board definitions, so that I can add support for new hardware.

#### Acceptance Criteria

1. WHEN a user runs `zigroot board new <name>`, THE Zigroot_CLI SHALL create a board template in `boards/<name>/` with `board.toml`
2. WHEN a user runs `zigroot verify <path>`, THE Zigroot_CLI SHALL validate the board structure, required fields, and TOML syntax (same command as package validation)
3. THE validation SHALL check that `board.toml` contains required fields: name, description, target, cpu
4. THE validation SHALL verify that the target is a valid Zig target triple
5. WHEN a user runs `zigroot publish <path>`, THE Zigroot_CLI SHALL detect whether the path contains a package or board and create a PR to the appropriate registry
6. THE publish command SHALL validate the board before creating the PR
7. THE publish command SHALL require GitHub authentication via `GITHUB_TOKEN` environment variable or `gh` CLI
8. WHEN publishing, THE Zigroot_CLI SHALL check that the board name doesn't conflict with existing boards (unless updating)


### Requirement 30: Minimum Zigroot Version

**User Story:** As a package/board maintainer, I want to specify a minimum zigroot version, so that users don't encounter errors from using outdated zigroot versions with newer packages.

#### Acceptance Criteria

1. A package definition MAY specify `zigroot_version` in `metadata.toml` to declare the minimum required zigroot version
2. A board definition MAY specify `zigroot_version` in `board.toml` to declare the minimum required zigroot version
3. THE `zigroot_version` field SHALL use semver syntax (e.g., `">=0.2.0"`, `"^1.0"`)
4. WHEN loading a package with `zigroot_version` specified, THE Zigroot_CLI SHALL compare against its own version
5. IF the current zigroot version does not satisfy the package's `zigroot_version` constraint, THEN THE Zigroot_CLI SHALL display an error with the required version and suggest updating
6. WHEN loading a board with `zigroot_version` specified, THE Zigroot_CLI SHALL compare against its own version
7. IF the current zigroot version does not satisfy the board's `zigroot_version` constraint, THEN THE Zigroot_CLI SHALL display an error with the required version and suggest updating
8. THE Zigroot_CLI SHALL follow semver standards for version comparison
9. WHEN a package uses features introduced in a specific zigroot version, THE package maintainer SHOULD set `zigroot_version` accordingly


### Requirement 31: Update Check

**User Story:** As a user, I want to know when a new version of zigroot is available, so that I can stay up to date with bug fixes and new features.

#### Acceptance Criteria

1. WHEN a user runs `zigroot update --self`, THE Zigroot_CLI SHALL check for newer versions of zigroot
2. IF a newer version is available, THEN THE Zigroot_CLI SHALL display the current version, latest version, and installation instructions
3. THE Zigroot_CLI MAY periodically check for updates in the background (at most once per day)
4. WHEN a background update check finds a newer version, THE Zigroot_CLI SHALL display a non-intrusive notification on the next command
5. THE update check SHALL query the GitHub releases API for `zigroot-project/zigroot-cli`
6. THE Zigroot_CLI SHALL cache the update check result to avoid repeated network requests
7. WHEN a user runs `zigroot update --self --install`, THE Zigroot_CLI SHALL attempt to download and install the latest version
8. THE self-update SHALL detect the installation method (cargo, homebrew, AUR, binary) and use the appropriate update mechanism
9. IF the installation method cannot be determined, THEN THE Zigroot_CLI SHALL display manual update instructions
10. THE Zigroot_CLI SHALL follow semver standards: major version changes indicate breaking changes, minor versions add features, patch versions fix bugs


### Requirement 32: Local Data Storage

**User Story:** As a user, I want zigroot to store cached data and settings in standard locations, so that I can manage disk space and configure global settings.

#### Acceptance Criteria

1. THE Zigroot_CLI SHALL store cached data in platform-specific cache directories (XDG on Linux, Library/Caches on macOS)
2. THE Zigroot_CLI SHALL store user configuration in platform-specific config directories
3. THE Zigroot_CLI SHALL store persistent data (downloads, build cache) in platform-specific data directories
4. THE Zigroot_CLI SHALL support environment variables to override default directories: `ZIGROOT_CACHE_DIR`, `ZIGROOT_CONFIG_DIR`, `ZIGROOT_DATA_DIR`
5. THE Zigroot_CLI SHALL read global settings from `config.toml` in the config directory
6. Global settings MAY include: registry URLs, cache TTL, default build options, update check settings, output preferences
7. WHEN a user runs `zigroot cache clean`, THE Zigroot_CLI SHALL clear the cache directory
8. WHEN a user runs `zigroot cache info`, THE Zigroot_CLI SHALL display cache size and location
9. THE Zigroot_CLI SHALL share downloaded source archives across projects in the data directory
10. THE Zigroot_CLI SHALL use content-addressable storage for build cache to enable sharing


### Requirement 33: Test-Driven Development

**User Story:** As a developer, I want a strict TDD workflow, so that all code is verified by tests before implementation and regressions are prevented.

#### Acceptance Criteria

1. FOR ALL new functionality, THE developer SHALL write failing tests before writing implementation code (red-green-refactor cycle)
2. THE test suite SHALL achieve minimum 80% line coverage for core modules (core/, registry/, infra/)
3. THE test suite SHALL achieve 100% coverage for all correctness properties defined in the design document
4. WHEN a bug is found, THE developer SHALL write a failing test that reproduces it before implementing the fix
5. THE CI pipeline SHALL fail if test coverage drops below the minimum threshold
6. FOR ALL public API functions in core modules, THE developer SHALL write at least one unit test
7. FOR ALL error conditions defined in requirements, THE developer SHALL write tests verifying correct error messages and behavior
8. THE test suite SHALL include property-based tests for all serialization round-trip requirements
9. THE test suite SHALL include integration tests for all CLI commands
10. WHEN refactoring code, THE developer SHALL ensure all existing tests pass before and after changes
11. THE developer SHALL use mock traits to isolate unit tests from external dependencies (network, filesystem, processes)
12. FOR ALL TOML parsing functions, THE developer SHALL include tests with malformed input to verify error handling

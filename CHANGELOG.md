# Changelog

## [0.1.0] - 2026-01-14

### Added

- Initialize Rust project with core architecture
- Implement option validation and resolution logic
- Implement download, git, and registry infrastructure
- Implement zigroot init command with full project initialization
- Implement zigroot add command with dependency resolution
- Implement zigroot remove command with package removal logic
- Implement zigroot update command with version checking
- Implement zigroot fetch command with package downloading
- Implement zigroot build command with package compilation
- Implement zigroot clean command with artifact removal
- Implement zigroot check command with validation
- Implement zigroot search command with unified package and board search
- Implement zigroot package subcommands with list and info
- Implement zigroot board subcommands with list, set, and info
- Implement zigroot tree command with dependency visualization
- Implement zigroot flash command with device flashing
- Implement zigroot external command with artifact management
- Implement binary compression with UPX support
- Implement advanced commands (doctor, sdk, license, cache)
- Implement zigroot config command with interactive TUI
- Implement zigroot package new, test, bump and verify commands
- Implement zigroot publish command with registry support
- Implement zigroot board new command with template generation
- Implement GCC toolchain and kernel build support
- Implement build isolation with Docker/Podman sandbox support
- Implement version management with semver checking and self-update
- Implement colored output and progress indicators with quiet/json modes
- Implement local data storage with platform directories and global config
- Implement configuration management with env vars and inheritance

### Documentation

- Add comprehensive specification documents
- Add implementation status section to tasks
- Update GitHub organization name to zigroot-project
- Mark zigroot tree checkpoint as complete

### Miscellaneous

- Add comprehensive .gitignore file

### Refactored

- Apply rustfmt and clippy fixes across entire codebase

### Security

- Add comprehensive CI/CD and release automation workflows

### Testing

- Add integration tests for zigroot package bump
- Add BuildSummary and error suggestion tests


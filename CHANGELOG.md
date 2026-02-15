# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.0] - TBD

### Changed

**Complete rewrite: Native Rust process supervision**

ubermind v0.6 removes all dependencies on overmind and tmux, replacing them with native Rust process management:

- **Native process supervision**: Direct PID-based process management without external dependencies
- **Auto-restart**: Configurable crash recovery with retry limits
- **Log management**: Automatic log rotation with timestamped files
- **Live streaming**: Ring buffers for real-time log output
- **Unified daemon**: Single daemon handles both process supervision and web UI
- **No external dependencies**: No longer requires overmind or tmux installation

### Breaking Changes

- Configuration format and APIs may differ from v0.5
- Migration from v0.5 projects should be straightforward (same Procfile format)
- Users upgrading should review the new documentation

### For Users of v0.1-v0.5

Earlier versions of ubermind were thin wrappers around [overmind](https://github.com/DarthSim/overmind). Version 0.6 represents a complete architectural shift to native process management while maintaining the same user-facing Procfile format and CLI commands.

## [0.5.1] - 2025-02-14

### Added

- Shell autocomplete for bash, zsh, and fish
  - Completes commands: `start`, `stop`, `status`, `restart`, etc.
  - Completes project names from config
  - Completes flags: `--all`, `-a`, `--daemon`, etc.
  - Example: `ub start appli<tab>` → `ub start appligator`
  - Install script automatically downloads completion files
  - Completions available in `completions/` directory

## [0.4.0] - 2024-12-19

### Changed

**Configuration file names renamed for clarity**

- `~/.config/ubermind/services` → `~/.config/ubermind/projects`
  - Clarifies that each entry is a project directory with its own Procfile
  
- `~/.config/ubermind/Procfile` → `~/.config/ubermind/commands`
  - Distinguishes ubermind's config from actual project Procfiles
  - Uses Procfile format for standalone commands
  
- `~/.config/ubermind/proc/` → `~/.config/ubermind/_commands/`
  - Underscore prefix signals this is an internal/auto-generated directory

### Improved

- Clearer mental model: **projects** (mapped directories) vs **commands** (standalone entries)
- Better documentation explaining the two-layer architecture
- All user-facing output now uses consistent "projects" terminology
- UI displays "No projects configured" message
- Help text and error messages updated throughout

### Technical

- Renamed `load_config_services()` → `load_projects()`
- Renamed `load_procfile_services()` → `load_commands()`
- Added `projects_config_path()` helper function
- Updated UI backend to use projects config path
- No API changes - internal refactoring only

## [0.3.5] - 2024-01-XX

### Added

- Initial stable release
- Multi-project management with Procfile support
- Web UI for monitoring and control
- Built on overmind/tmux (later replaced in v0.6)
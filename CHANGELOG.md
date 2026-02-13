# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
- Multi-project overmind management
- Web UI for monitoring and control
- Auto-installation of dependencies (overmind, tmux)
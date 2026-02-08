# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.11] - 2024

### Added
- PUB/SUB socket for broadcasting state changes after mutations.
- External integrations can now subscribe to real-time state updates.

### Changed
- Updated dependencies.

## [0.1.10] - 2024

### Changed
- Updated dependencies.

## [0.1.9] - 2024

### Changed
- Reviewed and improved client-server communication.

## [0.1.8] - 2024

### Added
- Send and receive timeouts on IPC sockets (500ms send, 5s receive) to prevent indefinite blocking.

## [0.1.7] - 2024

### Added
- Dependabot configuration for automated dependency updates.

### Changed
- Switched message serialization from custom format to JSON.
- Separated crate into `client`, `server`, and `bin-cli` feature flags for modular compilation.
- Updated dependencies.
- Improved release workflow with dasel for version checking.

### Removed
- `oclock_sqlite` lock file (now managed at workspace level).

## [0.1.6] - 2024

### Added
- Command and argument documentation via clap derive doc strings.
- Quick start section in README.

### Changed
- Replaced `getopts` with `clap` (derive) for CLI argument parsing.

## [0.1.5] - 2023

### Added
- GitHub Actions test workflow.
- GitHub Actions release build workflow with cross-compilation (Linux, Windows 32/64-bit, macOS).
- Publishing to crates.io as part of the release pipeline.
- Package metadata for crates.io (`description`, `keywords`, `license`, `repository`).

## [v0.1.4]

### Added
- Pivot timesheet format (tasks as columns, days as rows).
- `format_time_interval` function with unit tests.

### Changed
- Hide Shutdown and Ping events from timesheet output (only Startup events appear as idle markers).

## [v0.1.3]

### Added
- `current-task` command to query the active task.
- `json-retro-switch-task` command for retroactive time entries.
- `json-state` command returning current task and full task list as JSON.
- Moved `SystemEventType` to `oclock_sqlite` library for reuse.

## [v0.1.2]

### Changed
- Updated dependencies.

## [v0.1.1]

### Added
- `disable-task` command for soft-deleting tasks.
- JSON command variants (`json-push-task`, `json-disable-task`, `json-switch-task`) for integration with external tools (e.g. KDE Plasmoid).
- Error output redirected to stderr; exit code reflects success/failure.

### Fixed
- Stale Ping event removal on startup.
- Graceful shutdown on SIGTERM.

## [v0.1.0] - Initial development

### Added
- Client-server architecture using nanomsg (later migrated to NNG).
- SQLite database for event storage.
- Task creation, switching, and listing.
- Timesheet generation from database views.
- Single-threaded command executor with periodic ping.
- State initialization and session repair on startup.
- Logging support.
- GitLab CI pipeline.

# Contributing to Oclock

Thank you for your interest in contributing to Oclock! This document covers how to set up a development environment, build the project, run tests, and submit changes.

## Prerequisites

- **Rust** (stable toolchain) -- install via [rustup](https://rustup.rs/)
- **SQLite development headers** -- required by the `diesel` crate
  - Debian/Ubuntu: `sudo apt-get install libsqlite3-dev`
  - Fedora: `sudo dnf install sqlite-devel`
  - macOS: included with Xcode Command Line Tools
  - Arch Linux: `sudo pacman -S sqlite`

## Building

```sh
# Full build (client + server + CLI)
cargo build --all-features

# Release build
cargo build --release --all-features

# Build only the client library
cargo build --features client

# Build only the server library
cargo build --features server
```

## Running tests

```sh
cargo test --verbose
```

Tests are also run automatically on every push and pull request via GitHub Actions.

## Project structure

The workspace contains all crates under `crates/`:

| Crate | Path | Purpose |
|---|---|---|
| `oclock` | `crates/oclock/` | CLI binary, client handler, server daemon |
| `oclock-sqlite` | `crates/oclock-sqlite/` | SQLite data access (Diesel ORM, migrations, models) |

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for a detailed breakdown.

## Development workflow

1. **Fork and clone** the repository.
2. **Create a branch** for your change: `git checkout -b my-feature`.
3. **Make your changes** and add tests where appropriate.
4. **Run tests** locally: `cargo test --verbose`.
5. **Commit** with a clear, descriptive message.
6. **Open a pull request** against `master`.

## Database migrations

Oclock uses [Diesel migrations](https://diesel.rs/guides/getting-started.html) for schema changes. Migrations live in `crates/oclock-sqlite/migrations/`.

To add a new migration:

```sh
cd crates/oclock-sqlite
diesel migration generate <migration_name>
```

Write the SQL in the generated `up.sql` and `down.sql` files. Migrations are embedded at compile time and run automatically on daemon startup.

## Coding conventions

- Follow standard Rust formatting: `cargo fmt` before committing.
- Use `cargo clippy` to catch common issues.
- Keep feature flags in mind -- code behind `#[cfg(feature = "...")]` gates is only compiled when that feature is active.
- Error messages should be descriptive and logged at the appropriate level (`log::error!`, `log::warn!`, `log::debug!`).

## Release process

Releases are triggered by pushing a version tag:

```sh
git tag 0.1.12
git push origin 0.1.12
```

The GitHub Actions `build` workflow will:

1. Run tests.
2. Cross-compile for Linux, Windows (32/64-bit), and macOS.
3. Upload binaries to a GitHub release.
4. Publish `oclock_sqlite` and `oclock` to crates.io (if the version is new).

Remember to update the version in both `crates/oclock/Cargo.toml` and `crates/oclock-sqlite/Cargo.toml` as needed.

## Reporting issues

Please open an issue on [GitHub](https://github.com/dghilardi/oclock/issues) with:

- A clear description of the problem or feature request.
- Steps to reproduce (for bugs).
- Your OS and Rust version (`rustc --version`).

## License

By contributing, you agree that your contributions will be licensed under the [GPL-3.0-or-later](https://www.gnu.org/licenses/gpl-3.0) license.

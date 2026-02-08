# Oclock

[![Test Status](https://github.com/dghilardi/oclock/workflows/Tests/badge.svg?event=push)](https://github.com/dghilardi/oclock/actions)
[![Crate](https://img.shields.io/crates/v/oclock.svg)](https://crates.io/crates/oclock)
[![License: GPL-3.0-or-later](https://img.shields.io/crates/l/oclock)](https://www.gnu.org/licenses/gpl-3.0)

A lightweight, daemon-based time tracking utility written in Rust. Oclock uses a client-server architecture to let you create tasks, switch between them, and generate timesheets -- all from the command line.

## Features

- **Daemon-based** -- the server runs in the background and persists state across client invocations
- **Task management** -- create, list, switch between, and disable tasks
- **Automatic time tracking** -- timestamps are recorded on every task switch
- **Retroactive entries** -- correct past time records without losing current context
- **Timesheet generation** -- daily aggregated CSV output of time spent per task
- **Real-time state updates** -- PUB/SUB socket for external integrations (e.g. desktop widgets)
- **SQLite storage** -- durable, portable event log with zero configuration
- **JSON API** -- structured output variants for scripting and tool integration

## Installation

### From crates.io

```sh
cargo install oclock
```

### From source

```sh
git clone https://github.com/dghilardi/oclock.git
cd oclock
cargo build --release --all-features
```

The binary will be at `target/release/oclock`.

### Pre-built binaries

Linux binaries are attached to each [GitHub release](https://github.com/dghilardi/oclock/releases).

## Quick start

**1. Start the daemon**

```sh
oclock server
```

The server creates its database at `~/.oclock/oclock.db` and listens on local IPC sockets. Keep this running in a terminal or launch it as a background service.

**2. Create some tasks**

```sh
oclock client push-task --name "Development"
oclock client push-task --name "Code Review"
oclock client push-task --name "Meetings"
```

**3. Start tracking**

```sh
oclock client switch-task --task-id 1
```

**4. Check what you're working on**

```sh
oclock client current-task
```

**5. List all tasks**

```sh
oclock client list-tasks
```

**6. Generate a timesheet**

```sh
oclock client timesheet
```

This outputs a CSV with daily totals per task, formatted as `HH:MM:SS`.

**7. Stop the daemon**

```sh
oclock client exit
```

The daemon also shuts down gracefully on `Ctrl-C` or `SIGTERM`.

## Command reference

| Command | Description |
|---|---|
| `server` | Start the daemon |
| `client exit` | Shut down the daemon |
| `client push-task --name NAME` | Create a new task |
| `client disable-task --task-id ID` | Disable a task (soft-delete) |
| `client switch-task --task-id ID` | Switch to a task and start tracking |
| `client current-task` | Print the currently active task |
| `client list-tasks` | List all tasks (CSV) |
| `client timesheet` | Generate a full timesheet (CSV) |

### JSON variants

These commands return structured JSON, useful for scripting or integrations:

| Command | Description |
|---|---|
| `client json-push-task --name NAME` | Create a task, return full state |
| `client json-disable-task --task-id ID` | Disable a task, return full state |
| `client json-switch-task --task-id ID` | Switch task, return full state |
| `client json-retro-switch-task --task-id ID --timestamp TS [--keep-previous-task]` | Insert a past task switch |
| `client json-state` | Return current state as JSON |

The JSON state object looks like:

```json
{
  "current_task": { "id": 1, "enabled": 1, "name": "Development" },
  "all_tasks": [
    { "id": 1, "enabled": 1, "name": "Development" },
    { "id": 2, "enabled": 1, "name": "Code Review" },
    { "id": 3, "enabled": 1, "name": "Meetings" }
  ]
}
```

### Retroactive entries

If you forgot to switch tasks at the right time, use `json-retro-switch-task`:

```sh
# Record that you were on task 2 starting at unix timestamp 1700000000,
# then automatically switch back to whatever you were doing before
oclock client json-retro-switch-task --task-id 2 --timestamp 1700000000 --keep-previous-task
```

## Configuration

Oclock uses convention over configuration. The defaults are:

| Setting | Value |
|---|---|
| Database path | `~/.oclock/oclock.db` |
| IPC request socket | `ipc:///tmp/time-monitor.ipc` |
| IPC publish socket | `ipc:///tmp/time-monitor-sub.ipc` |

Logging verbosity is controlled via the standard `RUST_LOG` environment variable:

```sh
RUST_LOG=debug oclock server
```

## Library usage

Oclock can also be used as a Rust library. The crate exposes separate feature flags to include only what you need:

| Feature | Description |
|---|---|
| `api` | Data types and serialization (`OClockClientCommand`, models) |
| `client` | Client module for communicating with a running daemon |
| `server` | Server module (daemon logic, database, scheduling) |

```toml
[dependencies]
oclock = { version = "0.1", features = ["client"] }
```

```rust
use oclock::client::handler::invoke_server;
use oclock::dto::command::OClockClientCommand;

fn main() {
    let cmd = OClockClientCommand::JsonState;
    let reply: Result<serde_json::Value, _> = invoke_server(cmd);
    println!("{:?}", reply);
}
```

## How it works

Oclock follows an **event-sourcing** design. Every action (task switch, startup, shutdown) is recorded as an immutable event with a Unix timestamp in a SQLite database. Derived views (`v_history`, `v_timesheet`) compute durations and daily aggregates from the raw event log.

Communication between client and server uses [NNG](https://nng.nanomsg.org/) (nanomsg-next-generation) IPC sockets:

- **REQ/REP** socket for request-response commands
- **PUB** socket for broadcasting state changes to subscribers

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for a detailed breakdown.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines.

## License

This project is licensed under the [GNU General Public License v3.0 or later](https://www.gnu.org/licenses/gpl-3.0).

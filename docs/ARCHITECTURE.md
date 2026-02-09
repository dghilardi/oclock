# Architecture

This document describes the internal architecture of Oclock.

## Overview

Oclock is a client-server time tracking application. A long-running **server** (daemon) owns the SQLite database and listens on IPC sockets. Short-lived **client** processes send commands and print responses.

```
┌────────┐  JSON/IPC (REQ/REP)  ┌────────────┐
│ Client ├─────────────────────►│   Server   │
│ (CLI)  │◄─────────────────────┤  (Daemon)  │
└────────┘                      │            │
                                │  ┌───────┐ │
┌────────────┐  JSON/IPC (PUB)  │  │SQLite │ │
│ Subscriber ├◄─────────────────┤  │  DB   │ │
│ (widget)   │                  │  └───────┘ │
└────────────┘                  └────────────┘
```

## Crate structure

The project is a Cargo workspace. All crates live under `crates/`:

```
crates/oclock/           -- CLI binary + client/server library
crates/oclock-sqlite/    -- SQLite data access layer
```

### Feature flags

The `oclock` crate uses feature flags to compile only the required modules:

| Feature | Enables | Use case |
|---|---|---|
| `api` | `serde` | Shared data types for serialization |
| `client` | `nng`, `api`, `serde_json` | Sending commands to a running daemon |
| `server` | `nng`, `api`, `serde_json`, `schedule`, `itertools`, `csv`, `ctrlc`, `oclock_sqlite` | Running the daemon |
| `bin-cli` | `clap`, `env_logger` | Building the `oclock` binary |

Building with `--all-features` produces the full binary with both client and server capabilities.

## Module layout

```
crates/oclock/
├── src/
│   ├── lib.rs                      # Feature-gated module declarations
│   ├── core/
│   │   └── constants.rs            # IPC socket URL constants
│   ├── dto/
│   │   └── command.rs              # OClockClientCommand enum (shared protocol)
│   ├── client/
│   │   └── handler.rs              # invoke_server() -- sends a command, returns the reply
│   ├── server/
│   │   ├── handler.rs              # server() -- main daemon loop
│   │   └── state.rs                # State struct -- business logic over the database
│   └── bin/oclock/
│       ├── main.rs                 # CLI entry point
│       └── cli/
│           └── args.rs             # Clap argument definitions
└── Cargo.toml

crates/oclock-sqlite/
├── src/
│   ├── lib.rs
│   ├── connection.rs               # DB struct -- connection + migration management
│   ├── constants.rs                # SystemEventType enum
│   ├── models.rs                   # Diesel ORM models (Task, Event, TimesheetEntry, etc.)
│   ├── schema.rs                   # Diesel table! declarations
│   └── mappers/
│       ├── events.rs               # Event CRUD operations
│       ├── tasks.rs                # Task CRUD operations
│       └── timesheet.rs            # Timesheet query operations
├── migrations/                     # SQL migration files
└── Cargo.toml
```

## Communication protocol

### Transport

Client-server communication uses [NNG](https://nng.nanomsg.org/) IPC sockets:

- **REQ/REP** (`ipc:///tmp/time-monitor.ipc`) -- the client sends a request and waits for a single reply.
- **PUB** (`ipc:///tmp/time-monitor-sub.ipc`) -- the server publishes state snapshots after every mutation. External tools (e.g. desktop widgets) can subscribe to this socket for real-time updates.

### Message format

**Request:** JSON-serialized `OClockClientCommand`. The enum is tagged with `"cmd"` using screaming-snake-case variant names and camelCase fields:

```json
{ "cmd": "SWITCH_TASK", "taskId": 1 }
```

**Response:** UTF-8 string with a prefix and a `#` separator:

- `OK#<json_payload>` -- success
- `ERR#<error_message>` -- failure

### Timeouts

| Direction | Timeout |
|---|---|
| Server send | 500 ms |
| Server receive | 5 s |

The server uses non-blocking receive so the main scheduling loop remains responsive.

## Server internals

### Main loop

The daemon's main loop (`server::handler::server()`) uses a job scheduler that ticks every 300 ms:

1. **Every second** -- check the IPC socket for incoming messages and dispatch them.
2. **Every minute** -- update the `Ping` event timestamp.

A Ctrl-C handler sets an `AtomicBool` flag. The main loop checks this flag on every tick and exits cleanly when set.

### State management

`State` wraps the `DB` connection and provides the business-logic API:

- `new_task(name)` -- insert a task row
- `switch_task(id)` -- insert a task-switch event at the current time
- `retro_switch_task(id, timestamp, keep_prev)` -- insert a past event, optionally re-switch to the previous task
- `list_tasks()` / `get_current_task()` / `get_state()` -- read queries
- `change_task_enabled_flag(id, enabled)` -- soft-delete
- `system_event(type)` / `ping()` -- lifecycle tracking
- `full_timesheet()` -- pivot query that returns `(task_names, records)` for CSV generation

### Initialization

On startup, the `initialize()` function:

1. Reads the last event from the database.
2. If it is *not* a `Shutdown` event, inserts a synthetic `Shutdown` event at the same timestamp to close the previous incomplete session.
3. Removes all stale `Ping` events.

This ensures the event log is always well-formed regardless of how the previous session ended.

## Data model

Oclock uses an **event-sourcing** pattern. All state changes are appended as immutable `Event` rows. Derived state (durations, timesheets) is computed from database views.

### Event types

| `task_id` | `system_event_name` | Meaning |
|---|---|---|
| non-null | null | User switched to a task |
| null | `Startup` | Daemon started |
| null | `Shutdown` | Daemon stopped |
| null | `Ping` | Daemon is alive (updated in-place every minute) |

The `CHECK` constraint ensures at least one of `task_id` or `system_event_name` is always set.

### Duration computation

The `v_history` view computes durations by finding the next event's timestamp:

```
duration = next_event.timestamp - current_event.timestamp
```

If there is no next event (the event is the most recent), the duration is null and it is excluded from timesheets.

### Timesheet aggregation

The `v_timesheet` view groups `v_history` by `(day, task_id)` and sums durations. Days are derived using SQLite's `date(timestamp, 'unixepoch', 'localtime')` to respect the user's timezone.

## Error handling

- The `SrvInvocationError` enum (in `client::handler`) distinguishes between server-side errors and communication failures.
- Database errors from Diesel are mapped to `String` at the `State` boundary and propagated as `ERR#` responses.
- The server logs errors via the `log` crate but does not crash on individual request failures.

## Build and release

See [CONTRIBUTING.md](../CONTRIBUTING.md) for build instructions. The CI pipeline:

1. Runs `cargo test` on every push.
2. On version tags (`*.*.*`), cross-compiles for Linux/Windows/macOS, uploads GitHub release artifacts, and publishes to crates.io.

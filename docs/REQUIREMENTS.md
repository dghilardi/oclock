# Requirements

This document describes the functional and non-functional requirements for the Oclock time tracking utility.

## Functional requirements

### FR-1: Task management

| ID | Requirement | Status |
|---|---|---|
| FR-1.1 | Users can create named tasks | Implemented |
| FR-1.2 | Task names must be unique (case-insensitive) | Implemented |
| FR-1.3 | Users can list all registered tasks | Implemented |
| FR-1.4 | Users can disable (soft-delete) a task | Implemented |
| FR-1.5 | Each task has a numeric identifier assigned at creation | Implemented |

### FR-2: Time tracking

| ID | Requirement | Status |
|---|---|---|
| FR-2.1 | Users can switch the active task by ID; a timestamped event is recorded | Implemented |
| FR-2.2 | Users can query which task is currently active | Implemented |
| FR-2.3 | Users can insert a retroactive task switch at a past timestamp | Implemented |
| FR-2.4 | Retroactive switches can optionally preserve the previously active task by inserting a follow-up switch at the current time | Implemented |
| FR-2.5 | Time is computed as the difference between consecutive events (event-sourcing model) | Implemented |

### FR-3: Reporting

| ID | Requirement | Status |
|---|---|---|
| FR-3.1 | Users can generate a full timesheet as CSV output | Implemented |
| FR-3.2 | The timesheet aggregates time per task per day | Implemented |
| FR-3.3 | Time values are formatted as `HH:MM:SS` | Implemented |
| FR-3.4 | System events (Shutdown, Ping) are excluded from the timesheet; Startup events mark idle time boundaries | Implemented |

### FR-4: Daemon lifecycle

| ID | Requirement | Status |
|---|---|---|
| FR-4.1 | The server runs as a long-lived daemon process | Implemented |
| FR-4.2 | On startup, the daemon records a `Startup` event and initializes the database | Implemented |
| FR-4.3 | On shutdown (via `exit` command, `Ctrl-C`, or `SIGTERM`), the daemon records a `Shutdown` event | Implemented |
| FR-4.4 | The daemon periodically records a `Ping` event (every minute) to signal liveness | Implemented |
| FR-4.5 | On startup, if the previous session did not shut down cleanly, a synthetic `Shutdown` event is inserted to close the gap | Implemented |

### FR-5: Client-server communication

| ID | Requirement | Status |
|---|---|---|
| FR-5.1 | The client sends commands to the server and prints the response | Implemented |
| FR-5.2 | Commands are serialized as JSON over IPC (NNG REQ/REP sockets) | Implemented |
| FR-5.3 | Responses use `OK#<payload>` / `ERR#<message>` framing | Implemented |
| FR-5.4 | The server publishes state updates on a PUB socket after every mutation | Implemented |
| FR-5.5 | Commands have both human-readable (plain text/CSV) and machine-readable (JSON) variants | Implemented |

### FR-6: Data persistence

| ID | Requirement | Status |
|---|---|---|
| FR-6.1 | All data is stored in a local SQLite database | Implemented |
| FR-6.2 | Database migrations run automatically on startup | Implemented |
| FR-6.3 | Foreign key constraints are enforced | Implemented |

## Non-functional requirements

### NFR-1: Performance

| ID | Requirement |
|---|---|
| NFR-1.1 | The daemon must respond to client commands within the 5-second receive timeout |
| NFR-1.2 | The main loop polls at 300ms intervals, keeping CPU usage minimal when idle |
| NFR-1.3 | IPC communication must not block the daemon's scheduling loop for more than one iteration |

### NFR-2: Reliability

| ID | Requirement |
|---|---|
| NFR-2.1 | Incomplete sessions from crashes must be automatically repaired on next startup (synthetic Shutdown event) |
| NFR-2.2 | Stale Ping events are cleaned up on every startup to prevent database bloat |
| NFR-2.3 | All database writes use Diesel's type-safe query builder to prevent SQL injection |

### NFR-3: Portability

| ID | Requirement |
|---|---|
| NFR-3.1 | The application must compile and run on Linux |
| NFR-3.2 | Cross-compilation targets include Windows (x86, x86_64) and macOS (x86_64) |
| NFR-3.3 | IPC socket paths use platform-appropriate conventions (`ipc:///tmp/...`) |

### NFR-4: Usability

| ID | Requirement |
|---|---|
| NFR-4.1 | The CLI must provide `--help` for all commands and subcommands (via clap derive) |
| NFR-4.2 | Error messages must be printed to stderr; normal output to stdout |
| NFR-4.3 | The process exit code must reflect success (0) or failure (1) |

### NFR-5: Maintainability

| ID | Requirement |
|---|---|
| NFR-5.1 | The crate uses Cargo feature flags to separate client, server, and API concerns |
| NFR-5.2 | The SQLite layer is isolated in its own crate (`oclock_sqlite`) with a clean public API |
| NFR-5.3 | Database schema changes are managed through Diesel migrations |

## Database schema

### Tables

**tasks**

| Column | Type | Constraints |
|---|---|---|
| `id` | INTEGER | PRIMARY KEY, NOT NULL |
| `name` | VARCHAR | NOT NULL, UNIQUE (case-insensitive) |
| `enabled` | INTEGER | NOT NULL, DEFAULT 1 |

**events**

| Column | Type | Constraints |
|---|---|---|
| `id` | INTEGER | PRIMARY KEY, NOT NULL |
| `event_timestamp` | INTEGER | NOT NULL (Unix timestamp) |
| `task_id` | INTEGER | FK -> tasks(id), nullable |
| `system_event_name` | VARCHAR | nullable |

Constraint: at least one of `task_id` or `system_event_name` must be non-null.

### Views

**v_history** -- computes start/end timestamps and duration for each event by looking up the next event's timestamp.

**v_timesheet** -- aggregates `v_history` by day and task, summing durations. Groups by `(day, task_id, task_name, system_event)`.

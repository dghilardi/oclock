# OClock UI — Implementation Plan

This document describes the technical approach, project structure, technology choices, and phased implementation plan for the OClock desktop GUI.

## Technology decisions

### UI framework: Iced

[Iced](https://github.com/iced-rs/iced) is a cross-platform GUI library for Rust inspired by Elm.

**Why Iced:**

- Pure Rust, compiles to a single binary.
- Elm architecture (Model → View → Update) maps naturally to OClock's event-sourcing model: daemon state changes arrive as messages and the UI re-renders.
- Built-in `Canvas` widget for custom drawing (timeline visualization).
- Built-in `Subscription` mechanism for integrating async event sources (PUB socket, idle detection).
- GPU-accelerated rendering via wgpu.
- Active development and growing ecosystem.

**Risks and mitigations:**

| Risk | Mitigation |
|---|---|
| No native system tray support | Use `ksni` crate (DBus StatusNotifierItem) for Linux tray. Iced window is managed separately. |
| Not using native OS widgets | Acceptable — the app has a custom data-centric UI, not a forms-heavy one. |
| Canvas API learning curve for timeline | Start with a simple day view; iterate on weekly/monthly later. |

**Fallback:** If Iced proves insufficient for complex visualizations, Tauri (Rust backend + web frontend) is the secondary option. The `oclock-bridge` crate would be reusable in either case.

### System tray: ksni

[ksni](https://crates.io/crates/ksni) implements the StatusNotifierItem DBus protocol, which is supported by KDE, GNOME (via extensions), Sway, and most Linux desktop environments.

For future macOS/Windows support, `tray-icon` can be used behind a platform abstraction.

### Idle detection: platform backends

| Platform | Mechanism | Crate / API |
|---|---|---|
| X11 | `XScreenSaverQueryInfo` | `x11` crate or `xcb` |
| Wayland (wlroots, KDE) | `ext-idle-notify-v1` protocol | `wayland-client` + protocol extension |
| GNOME on Wayland | `org.gnome.Mutter.IdleMonitor` DBus | `zbus` |

The idle detector will be a trait with platform-specific implementations, selected at startup based on the session type (`$XDG_SESSION_TYPE` and compositor capabilities).

### Communication with daemon: oclock library

```toml
[dependencies]
oclock = { path = "../oclock", features = ["client", "api"] }
```

- **Commands**: call `oclock::client::handler::invoke_server()` from a background thread (it blocks on NNG sockets).
- **Subscriptions**: connect to the PUB socket (`ipc:///tmp/time-monitor-sub.ipc`) via NNG `Sub0` in a dedicated thread, forward messages as Iced `Subscription` events.
- **Initial state**: fetch via `JsonState` command on startup, then keep in sync via PUB.

## Project structure

The UI is a **separate repository** (not inside the oclock repo) with a Cargo workspace:

```
oclock-ui/
├── Cargo.toml                  # [workspace]
├── crates/
│   ├── oclock-bridge/          # Library: daemon communication + state management
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── client.rs       # Wraps invoke_server() with async interface
│   │       ├── subscriber.rs   # PUB socket listener, emits state events
│   │       └── state.rs        # Cached daemon state, updated by subscriber
│   │
│   ├── oclock-idle/            # Library: idle detection abstraction
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs          # IdleDetector trait + factory
│   │       ├── x11.rs          # X11 backend
│   │       ├── wayland.rs      # ext-idle-notify backend
│   │       └── gnome_dbus.rs   # GNOME Mutter backend
│   │
│   └── oclock-ui/              # Binary: the Iced application
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs         # Entry point, tray icon setup, Iced app launch
│           ├── app.rs          # Iced Application impl (Message, update, view)
│           ├── theme.rs        # Colors, task color assignment
│           ├── config.rs       # XDG config file (task colors, idle threshold, etc.)
│           ├── tray.rs         # ksni tray icon integration
│           ├── subscriptions.rs # Iced Subscriptions (daemon state, idle events)
│           ├── views/
│           │   ├── mod.rs
│           │   ├── quick_switch.rs   # Task list + switch (M1)
│           │   ├── timeline.rs       # Calendar-style visualization (M2)
│           │   ├── detail.rs         # Tabular event list + editing (M3)
│           │   └── export.rs         # Export options (M4)
│           └── dialogs/
│               ├── mod.rs
│               ├── idle_return.rs    # Idle return dialog (M1)
│               └── new_task.rs       # New task dialog (M1)
│
├── assets/
│   ├── icons/                  # Tray and app icons (SVG)
│   └── oclock-ui.desktop       # XDG desktop entry for autostart
│
└── README.md
```

### Crate dependency graph

```
oclock-ui (bin)
├── oclock-bridge (lib)
│   └── oclock (external, features = ["client", "api"])
├── oclock-idle (lib)
├── iced
└── ksni
```

## Architecture

### Data flow

```
                          ┌──────────────┐
                          │  Iced App    │
                          │  (update fn) │
                          └──┬───────┬───┘
                     Message │       │ Command
                             │       │
              ┌──────────────▼─┐   ┌─▼──────────────┐
              │  Subscription  │   │  Task::perform  │
              │  (PUB socket)  │   │  (invoke_server)│
              └──────────────┬─┘   └─┬──────────────┘
                             │       │
                       NNG   │       │  NNG
                      Sub0   │       │  Req0
                             │       │
                      ┌──────▼───────▼──────┐
                      │    oclock daemon    │
                      └─────────────────────┘
```

1. On startup, `oclock-bridge` sends a `JsonState` command to get the initial state.
2. A `Subscription` thread connects to the PUB socket and listens for state updates.
3. Every state update is forwarded as an Iced `Message::StateUpdated(ExportedState)`.
4. User actions (click task, create task, etc.) dispatch `Command`s that call `invoke_server()` via `Task::perform` (Iced's async command model).
5. The daemon processes the command, mutates state, and publishes the new state on PUB.
6. The subscription picks it up and the cycle repeats.

### Idle detection flow

```
                     ┌───────────────┐
                     │ IdleDetector   │
                     │ (Subscription) │
                     └───────┬───────┘
                             │
               IdleEvent::UserReturned { idle_since }
                             │
                     ┌───────▼───────┐
                     │   Iced App    │
                     │ show dialog   │
                     └───────┬───────┘
                             │ user confirms
                     ┌───────▼───────┐
                     │ RetroSwitch   │
                     │ Command       │
                     └───────────────┘
```

The idle detector runs as an Iced `Subscription`. It emits:
- `IdleEvent::IdleStarted { since: Instant }` — user went idle (UI may dim or update tooltip).
- `IdleEvent::UserReturned { idle_since: DateTime }` — user came back after exceeding the threshold; the app shows the idle-return dialog.

### Tray icon integration

The tray icon runs on its own thread (ksni requires a glib/tokio event loop). Communication between tray and Iced app happens via channels:

- **Tray → App**: `TrayMessage::ToggleWindow`, `TrayMessage::SwitchTask(id)`
- **App → Tray**: update tooltip text, update menu items

This decoupling means the tray icon works even when the main window is hidden.

### Configuration

Stored at `$XDG_CONFIG_HOME/oclock-ui/config.toml`:

```toml
[idle]
threshold_minutes = 5

[tasks]
# Local color overrides (task_id -> hex color)
[tasks.colors]
1 = "#4A90D9"
2 = "#E67E22"

[window]
start_minimized = true
```

Task colors that are not explicitly configured are auto-assigned from a palette.

## Implementation phases

### Phase 1 — Scaffolding and bridge (M1 foundation)

**Goal:** Set up the workspace, establish communication with the daemon, prove the architecture.

1. Create the Cargo workspace with `oclock-bridge`, `oclock-idle`, and `oclock-ui` crates.
2. Implement `oclock-bridge::client` — async wrapper around `invoke_server()` using `tokio::task::spawn_blocking`.
3. Implement `oclock-bridge::subscriber` — PUB socket listener that emits state events via a channel.
4. Implement `oclock-bridge::state` — cached `ExportedState` updated by the subscriber.
5. Create a minimal Iced app that displays the current task name (proof of concept).
6. Wire up the Iced `Subscription` for real-time state updates.

**Validation:** Launch the app alongside the daemon; switch tasks via CLI and confirm the UI updates in real time.

### Phase 2 — Quick switch view (M1 core)

**Goal:** Full task list with switching, creation, and deletion.

1. Build the quick switch view: scrollable task list with active-task highlighting.
2. Implement task switching on click (dispatch `JsonSwitchTask` command).
3. Add "new task" dialog with text input (dispatch `JsonPushTask`).
4. Add task disable button (dispatch `JsonDisableTask`).
5. Add navigation tabs/sidebar skeleton for future views.

**Validation:** All task management operations work from the UI; state stays in sync.

### Phase 3 — System tray (M1)

**Goal:** Tray icon with tooltip and context menu.

1. Integrate `ksni` in a background thread.
2. Show current task in tooltip.
3. Build context menu with task list for quick switching.
4. Implement left-click to toggle window visibility.
5. Add XDG `.desktop` file for autostart.

**Validation:** Tray icon appears on KDE, GNOME, and Sway; menu works; window toggles.

### Phase 4 — Idle detection (M1)

**Goal:** Detect idle time and prompt user on return.

1. Implement `IdleDetector` trait in `oclock-idle`.
2. Implement X11 backend (`XScreenSaverQueryInfo`).
3. Implement Wayland backend (`ext-idle-notify-v1`).
4. Implement GNOME DBus backend (`org.gnome.Mutter.IdleMonitor`).
5. Add runtime backend selection based on `$XDG_SESSION_TYPE` and compositor probing.
6. Wire up as Iced `Subscription`.
7. Build the idle-return dialog (task picker, "resume previous" checkbox).
8. Dispatch `JsonRetroSwitchTask` on confirmation.

**Validation:** Go idle for 5+ minutes, come back, confirm dialog appears and retroactive switch works.

### Phase 5 — Configuration (M1 polish)

**Goal:** Persistent user preferences.

1. Define `Config` struct with serde + toml.
2. Load/save from `$XDG_CONFIG_HOME/oclock-ui/config.toml`.
3. Wire idle threshold to config.
4. Wire `start_minimized` to config.
5. Implement task color assignment (explicit overrides + auto palette).

**Validation:** Change idle threshold in config, restart app, confirm new threshold is used.

### Phase 6 — Timeline view (M2)

**Goal:** Visual time blocks on a daily axis.

> **Prerequisite:** Daemon support for querying events by date range (BE-1).

1. Implement BE-1 in the daemon (new `JsonEventsByRange` command).
2. Add the command to `oclock-bridge`.
3. Build the daily timeline view using Iced `Canvas`:
   - Vertical time axis (00:00–24:00).
   - Colored rectangles for each time block.
   - Task name labels inside or beside blocks.
   - Legend below the canvas.
4. Add date navigation (previous/next day).
5. Extend to weekly view (7 columns) and monthly view (grid of days with summary bars).

### Phase 7 — Detail view (M3)

**Goal:** Tabular event listing with editing.

> **Prerequisite:** Daemon support for event mutation (BE-2, BE-3).

1. Implement BE-2 and BE-3 in the daemon.
2. Build the tabular view: sortable columns, date range filter.
3. Implement inline editing of event timestamp and task.
4. Implement event deletion with confirmation.
5. Implement manual event insertion.

### Phase 8 — Export (M4)

**Goal:** Export timesheet data with formatting options.

1. Build the export view: date range picker, format selector.
2. Implement CSV export (reuse daemon's timesheet or compute client-side from raw events).
3. Add rounding option (nearest N minutes).
4. Add grouping option (by task only, collapsing days).
5. File save dialog.

### Phase 9 — Integrations (M5)

**Goal:** Push time data to external systems.

1. Design a plugin trait: `Integration { fn push_time_entry(...) }`.
2. Implement Jira integration (REST API, worklog endpoint).
3. Build the integration configuration UI (API URL, credentials, task-to-issue mapping).
4. Add a "push to Jira" action in the export or detail view.

## Key dependencies

| Crate | Purpose | Version (indicative) |
|---|---|---|
| `iced` | GUI framework | 0.13+ |
| `ksni` | Linux system tray (StatusNotifierItem) | 0.2+ |
| `oclock` | Daemon communication (library) | 0.1 (path dependency) |
| `nng` | PUB socket subscription | 1.0 |
| `tokio` | Async runtime (for Iced commands) | 1 |
| `serde` + `toml` | Configuration file | 1.0 / 0.8 |
| `directories` | XDG path resolution | 5 |
| `x11` or `xcb` | X11 idle detection | — |
| `wayland-client` | Wayland idle detection | 0.31+ |
| `zbus` | DBus communication (GNOME idle, ksni internals) | 4+ |
| `chrono` | Date/time formatting | 0.4 |

## Open questions

1. **Separate repo vs monorepo:** The plan assumes a separate `oclock-ui` repo with a path dependency on oclock. Alternatively, oclock-ui could live inside the oclock repo as a workspace member. Separate repo is cleaner for independent release cycles, but monorepo simplifies development. **Decision needed.**

2. **Iced version:** Iced 0.13 is the current stable. The API is evolving — we should pin to a specific version and track upstream closely.

3. **NNG vs alternative IPC:** The daemon uses NNG. If NNG's Rust bindings become unmaintained, switching to Unix domain sockets with a simple framing protocol would be an option, but that requires daemon changes.

4. **Accessibility:** Iced's accessibility story is still maturing. For full keyboard navigation and screen reader support, this may need contributions upstream or workarounds.

5. **Wayland idle detection coverage:** `ext-idle-notify-v1` is not supported by all compositors. Need to verify coverage on common setups (KDE, Sway, Hyprland) and document unsupported ones.

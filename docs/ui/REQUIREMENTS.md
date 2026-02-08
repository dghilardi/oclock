# OClock UI — Product Requirements Document

This document defines the requirements for a standalone desktop GUI frontend for the OClock time tracking daemon.

## Background

OClock is a client-server time tracking application. A long-running daemon owns the SQLite database and communicates via NNG IPC sockets. The current desktop integration is a KDE Plasma 6 applet (`plasma-applet-oclock-control`) that shells out to the `oclock` CLI and depends on Plasma-specific APIs for idle detection and system tray presence.

The goal of this project is to build a **desktop-environment-agnostic** GUI that communicates with the daemon as a Rust library (not via CLI), supports idle detection on both X11 and Wayland, and provides a richer feature set including timeline visualization, event editing, and data export.

## Goals

- Replace the KDE-specific applet with a cross-desktop application.
- Use OClock as a library (`client` + `api` features) for direct IPC communication.
- Subscribe to the PUB socket for real-time state updates instead of polling.
- Provide a system tray icon for quick task switching.
- Detect user idle time on X11, Wayland (ext-idle-notify), and GNOME (DBus IdleMonitor).
- Lay the groundwork for advanced features: timeline view, event editing, export, third-party integrations.

## Non-goals

- Replacing or modifying the OClock daemon itself (backend changes are tracked separately).
- Mobile or web support.
- Embedding the daemon inside the UI process (the daemon stays on systemd).

## Functional requirements

### FR-UI-1: System tray

| ID | Requirement | Priority |
|---|---|---|
| FR-UI-1.1 | The application runs as a system tray icon using the StatusNotifierItem (SNI) DBus protocol on Linux | Must |
| FR-UI-1.2 | The tray icon tooltip shows the currently active task name | Must |
| FR-UI-1.3 | Left-click on the tray icon toggles the main window | Must |
| FR-UI-1.4 | Right-click opens a context menu listing all enabled tasks; clicking a task switches to it | Must |
| FR-UI-1.5 | The tray icon visually indicates whether a task is active or the tracker is idle (e.g. via icon color) | Should |

### FR-UI-2: Quick switch view

| ID | Requirement | Priority |
|---|---|---|
| FR-UI-2.1 | The main window shows a list of all enabled tasks | Must |
| FR-UI-2.2 | The currently active task is visually highlighted | Must |
| FR-UI-2.3 | Clicking a task switches to it immediately | Must |
| FR-UI-2.4 | A button or shortcut allows creating a new task inline | Must |
| FR-UI-2.5 | Tasks can be disabled (soft-deleted) from the list | Must |
| FR-UI-2.6 | The view updates in real time via PUB socket subscription | Must |

### FR-UI-3: Idle detection

| ID | Requirement | Priority |
|---|---|---|
| FR-UI-3.1 | The application detects user inactivity using platform-appropriate mechanisms (X11 XScreenSaver, Wayland ext-idle-notify-v1, GNOME DBus IdleMonitor) | Must |
| FR-UI-3.2 | The idle threshold is configurable (default: 5 minutes) | Must |
| FR-UI-3.3 | When the user returns from idle, a dialog appears asking what task they were working on during the idle period | Must |
| FR-UI-3.4 | The dialog shows the idle start time and lets the user pick from the task list | Must |
| FR-UI-3.5 | The user can optionally indicate they are resuming the previous task (triggers `retro_switch_task` with `keep_previous = true`) | Must |
| FR-UI-3.6 | If the user dismisses the dialog without action, no retroactive change is made | Must |

### FR-UI-4: Timeline visualization

| ID | Requirement | Priority |
|---|---|---|
| FR-UI-4.1 | A timeline view shows time blocks as colored rectangles on a time axis (similar to Google Calendar day view) | Should |
| FR-UI-4.2 | The view supports daily, weekly, and monthly granularity | Should |
| FR-UI-4.3 | Each task has a consistent color across all views | Should |
| FR-UI-4.4 | A legend maps task names to their colors | Should |
| FR-UI-4.5 | System events (Startup/Shutdown) are shown as visual markers or gaps | Could |

### FR-UI-5: Detail / event editing view

| ID | Requirement | Priority |
|---|---|---|
| FR-UI-5.1 | A tabular view lists all events with columns: timestamp, task name, duration | Should |
| FR-UI-5.2 | Events can be filtered by date range | Should |
| FR-UI-5.3 | Users can manually insert a new event at a given timestamp | Could |
| FR-UI-5.4 | Users can edit the timestamp or task of an existing event | Could |
| FR-UI-5.5 | Users can delete an event | Could |

> **Note:** FR-UI-5.3 through FR-UI-5.5 require new daemon commands (event mutation API). These are out of scope for the initial release and tracked as backend work.

### FR-UI-6: Task management

| ID | Requirement | Priority |
|---|---|---|
| FR-UI-6.1 | Users can create new tasks from the UI | Must |
| FR-UI-6.2 | Users can disable (soft-delete) tasks | Must |
| FR-UI-6.3 | Users can re-enable previously disabled tasks | Could |
| FR-UI-6.4 | Users can assign a color to a task (stored locally by the UI) | Should |

> **Note:** FR-UI-6.3 requires a new daemon command. FR-UI-6.4 can be stored in a local config file without daemon changes.

### FR-UI-7: Export

| ID | Requirement | Priority |
|---|---|---|
| FR-UI-7.1 | Users can export timesheet data as CSV | Should |
| FR-UI-7.2 | Users can choose a date range for the export | Should |
| FR-UI-7.3 | Export supports rounding to configurable intervals (e.g. nearest 30 minutes) | Could |
| FR-UI-7.4 | Export supports grouping by task only (collapsing days) | Could |

### FR-UI-8: Third-party integrations

| ID | Requirement | Priority |
|---|---|---|
| FR-UI-8.1 | A plugin or configuration system allows mapping OClock tasks to external system identifiers (e.g. Jira issue keys) | Could |
| FR-UI-8.2 | Users can push tracked time to Jira worklogs | Could |

> **Note:** Integrations are a future milestone and not in scope for the initial release.

## Non-functional requirements

### NFR-UI-1: Platform support

| ID | Requirement |
|---|---|
| NFR-UI-1.1 | The application must run on Linux (X11 and Wayland) |
| NFR-UI-1.2 | The application should be portable to macOS and Windows in the future |
| NFR-UI-1.3 | The application must not depend on any desktop-environment-specific library at runtime |

### NFR-UI-2: Performance

| ID | Requirement |
|---|---|
| NFR-UI-2.1 | The UI must remain responsive (<100ms) during daemon communication |
| NFR-UI-2.2 | State updates from the PUB socket must be reflected in the UI within 500ms |
| NFR-UI-2.3 | The idle process should consume minimal CPU when the window is not visible |

### NFR-UI-3: Distribution

| ID | Requirement |
|---|---|
| NFR-UI-3.1 | The application must compile to a single statically-linked binary (excluding system libraries) |
| NFR-UI-3.2 | A `.desktop` file must be provided for XDG autostart |
| NFR-UI-3.3 | The application should follow the XDG Base Directory specification for configuration files |

### NFR-UI-4: Usability

| ID | Requirement |
|---|---|
| NFR-UI-4.1 | The UI must respect the system color scheme (light/dark) where the toolkit supports it |
| NFR-UI-4.2 | All interactive elements must be keyboard-accessible |
| NFR-UI-4.3 | The idle dialog must be modal and brought to focus automatically |

## Daemon extension requirements

The following changes to the OClock daemon are needed to fully support the UI features listed above. They are tracked here for completeness but implemented separately.

| ID | Requirement | Needed by |
|---|---|---|
| BE-1 | Query events by date range (raw events, not aggregated timesheet) | FR-UI-4, FR-UI-5 |
| BE-2 | Update an existing event (change timestamp or task_id) | FR-UI-5.4 |
| BE-3 | Delete an existing event | FR-UI-5.5 |
| BE-4 | Re-enable a disabled task | FR-UI-6.3 |
| BE-5 | Query timesheet by date range (filtered aggregation) | FR-UI-7.2 |
| BE-6 | Export with rounding/grouping options (or expose raw data and let the UI compute) | FR-UI-7.3, FR-UI-7.4 |

## Milestones

### M1 — MVP (tray + quick switch + idle detection)

Deliver FR-UI-1, FR-UI-2, FR-UI-3, FR-UI-6.1, FR-UI-6.2. This replaces the Plasma applet with feature parity.

### M2 — Visualization

Deliver FR-UI-4 (timeline view). Requires BE-1.

### M3 — Detail editing

Deliver FR-UI-5. Requires BE-1, BE-2, BE-3.

### M4 — Export

Deliver FR-UI-7. May require BE-5, BE-6 depending on approach.

### M5 — Integrations

Deliver FR-UI-8. Design the plugin system and implement Jira as the first integration.

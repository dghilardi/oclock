use oclock::client::handler::{invoke_server, SrvInvocationError};
use oclock::dto::command::OClockClientCommand;
use oclock::dto::state::{ExportedState, TimeBlock};

/// Send a command to the daemon and return the new state.
///
/// This runs the blocking NNG call on the tokio blocking thread pool
/// so it is safe to call from async contexts.
pub async fn send_command(cmd: OClockClientCommand) -> Result<ExportedState, SrvInvocationError> {
    tokio::task::spawn_blocking(move || invoke_server::<_, ExportedState>(cmd))
        .await
        .expect("blocking task panicked")
}

/// Fetch the current daemon state.
pub async fn get_state() -> Result<ExportedState, SrvInvocationError> {
    send_command(OClockClientCommand::JsonState).await
}

/// Switch to a task by ID and return the new state.
pub async fn switch_task(task_id: u64) -> Result<ExportedState, SrvInvocationError> {
    send_command(OClockClientCommand::JsonSwitchTask { task_id }).await
}

/// Create a new task and return the new state.
pub async fn push_task(name: String) -> Result<ExportedState, SrvInvocationError> {
    send_command(OClockClientCommand::JsonPushTask { name }).await
}

/// Disable a task and return the new state.
pub async fn disable_task(task_id: u64) -> Result<ExportedState, SrvInvocationError> {
    send_command(OClockClientCommand::JsonDisableTask { task_id }).await
}

/// Get history entries (time blocks) within a timestamp range.
pub async fn events_by_range(
    start_timestamp: u64,
    end_timestamp: u64,
) -> Result<Vec<TimeBlock>, SrvInvocationError> {
    let cmd = OClockClientCommand::JsonEventsByRange {
        start_timestamp,
        end_timestamp,
    };
    tokio::task::spawn_blocking(move || invoke_server::<_, Vec<TimeBlock>>(cmd))
        .await
        .expect("blocking task panicked")
}

/// Delete an event by ID and return the new state.
pub async fn delete_event(event_id: u64) -> Result<ExportedState, SrvInvocationError> {
    send_command(OClockClientCommand::JsonDeleteEvent { event_id }).await
}

/// Edit an event's timestamp and/or task, return the new state.
pub async fn edit_event(
    event_id: u64,
    new_timestamp: Option<u64>,
    new_task_id: Option<Option<i32>>,
) -> Result<ExportedState, SrvInvocationError> {
    send_command(OClockClientCommand::JsonEditEvent {
        event_id,
        new_timestamp,
        new_task_id,
    })
    .await
}

/// Insert a manual event and return the new state.
pub async fn insert_event(
    timestamp: u64,
    task_id: Option<i32>,
) -> Result<ExportedState, SrvInvocationError> {
    send_command(OClockClientCommand::JsonInsertEvent { timestamp, task_id }).await
}

/// Retroactively switch task at a past timestamp.
pub async fn retro_switch_task(
    task_id: u64,
    timestamp: u64,
    keep_previous_task: bool,
) -> Result<ExportedState, SrvInvocationError> {
    send_command(OClockClientCommand::JsonRetroSwitchTask {
        task_id,
        timestamp,
        keep_previous_task,
    })
    .await
}

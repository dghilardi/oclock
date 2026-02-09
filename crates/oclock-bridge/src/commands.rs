use oclock::client::handler::{invoke_server, SrvInvocationError};
use oclock::dto::command::OClockClientCommand;
use oclock::dto::state::ExportedState;

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

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", tag = "cmd")]
pub enum OClockClientCommand {
    /// Terminate the server instance
    #[serde(rename_all = "camelCase")]
    Exit,
    /// Create a new task
    #[serde(rename_all = "camelCase")]
    PushTask { name: String },
    /// Disable the task with the given id
    #[serde(rename_all = "camelCase")]
    DisableTask { task_id: u64 },
    /// Switch to the task with the given id
    #[serde(rename_all = "camelCase")]
    SwitchTask { task_id: u64 },
    /// Read the current task
    #[serde(rename_all = "camelCase")]
    CurrentTask,
    /// List all registered tasks
    #[serde(rename_all = "camelCase")]
    ListTasks,
    /// Create a new task (json version)
    #[serde(rename_all = "camelCase")]
    JsonPushTask { name: String },
    /// Disable the task with the given id (json version)
    #[serde(rename_all = "camelCase")]
    JsonDisableTask { task_id: u64 },
    /// Switch to the task with the given id (json version)
    #[serde(rename_all = "camelCase")]
    JsonSwitchTask { task_id: u64 },
    /// Switch to the task with the given id at the given time, eventually returning to the current task (json version)
    #[serde(rename_all = "camelCase")]
    JsonRetroSwitchTask {
        task_id: u64,
        timestamp: u64,
        keep_previous_task: bool,
    },
    /// Read the current state (json version)
    #[serde(rename_all = "camelCase")]
    JsonState,
    /// Get history entries within a timestamp range (json version)
    #[serde(rename_all = "camelCase")]
    JsonEventsByRange {
        start_timestamp: u64,
        end_timestamp: u64,
    },
    /// Delete an event by ID (json version)
    #[serde(rename_all = "camelCase")]
    JsonDeleteEvent { event_id: u64 },
    /// Edit an event's timestamp and/or task (json version)
    #[serde(rename_all = "camelCase")]
    JsonEditEvent {
        event_id: u64,
        new_timestamp: Option<u64>,
        new_task_id: Option<Option<i32>>,
    },
    /// Insert a manual event (json version)
    #[serde(rename_all = "camelCase")]
    JsonInsertEvent {
        timestamp: u64,
        task_id: Option<i32>,
    },
    /// Produce the full timesheet
    #[serde(rename_all = "camelCase")]
    Timesheet,
}

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
    /// Produce the full timesheet
    #[serde(rename_all = "camelCase")]
    Timesheet,
}

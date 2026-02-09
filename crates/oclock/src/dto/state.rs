use serde::{Deserialize, Serialize};

/// A task as exposed over the wire.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TaskInfo {
    pub id: i32,
    pub enabled: i32,
    pub name: String,
}

/// Daemon state snapshot published on the PUB socket and returned by `JsonState`.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExportedState {
    pub current_task: Option<TaskInfo>,
    pub all_tasks: Vec<TaskInfo>,
}

impl ExportedState {
    pub fn new(current_task: Option<TaskInfo>, all_tasks: Vec<TaskInfo>) -> Self {
        Self {
            current_task,
            all_tasks,
        }
    }
}

/// A single time block in the history, used by the timeline view.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TimeBlock {
    pub ts_start: i64,
    pub ts_end: Option<i64>,
    pub task_id: Option<i32>,
    pub task_name: Option<String>,
}

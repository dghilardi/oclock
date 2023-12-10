use std::path::PathBuf;
use clap::{Args, Parser, Subcommand};

/// Simple time tracking software
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct OClockArgs {
    #[arg(short, long)]
    pub path: Option<PathBuf>,
    #[clap(subcommand)]
    pub subcommand: OClockCommand,
}

#[derive(Subcommand, Debug)]
pub enum OClockCommand {
    #[cfg(feature = "server")]
    /// Launch oclock in server mode
    Server,
    #[cfg(feature = "client")]
    /// Launch oclock in client mode
    Client(ClientArgs)
}

#[derive(Args, Debug)]
pub struct ClientArgs {
    #[clap(subcommand)]
    pub command: OClockClientCommandArg,
}

#[derive(Subcommand, Debug)]
pub enum OClockClientCommandArg {
    /// Terminate the server instance
    Exit,
    /// Create a new task
    PushTask {
        #[clap(long, short)]
        name: String
    },
    /// Disable the task with the given id
    DisableTask {
        #[clap(long, short)]
        task_id: u64
    },
    /// Switch to the task with the given id
    SwitchTask {
        #[clap(long, short)]
        task_id: u64
    },
    /// Read the current task
    CurrentTask,
    /// List all registered tasks
    ListTasks,
    /// Create a new task (json version)
    JsonPushTask {
        #[clap(long, short)]
        name: String
    },
    /// Disable the task with the given id (json version)
    JsonDisableTask {
        #[clap(long, short)]
        task_id: u64
    },
    /// Switch to the task with the given id (json version)
    JsonSwitchTask {
        #[clap(long, short)]
        task_id: u64
    },
    /// Switch to the task with the given id at the given time, eventually returning to the current task (json version)
    JsonRetroSwitchTask {
        #[clap(long)]
        task_id: u64,
        #[clap(long)]
        timestamp: u64,
        #[clap(long, short)]
        keep_previous_task: bool,
    },
    /// Read the current state (json version)
    JsonState,
    /// Produce the full timesheet
    Timesheet,
}

#[cfg(feature = "api")]
impl From<OClockClientCommandArg> for oclock::dto::command::OClockClientCommand {
    fn from(value: OClockClientCommandArg) -> Self {
        match value {
            OClockClientCommandArg::Exit => Self::Exit,
            OClockClientCommandArg::PushTask { name } => Self::PushTask { name },
            OClockClientCommandArg::DisableTask { task_id } => Self::DisableTask { task_id },
            OClockClientCommandArg::SwitchTask { task_id } => Self::SwitchTask { task_id },
            OClockClientCommandArg::CurrentTask => Self::CurrentTask,
            OClockClientCommandArg::ListTasks => Self::ListTasks,
            OClockClientCommandArg::JsonPushTask { name } => Self::JsonPushTask { name },
            OClockClientCommandArg::JsonDisableTask { task_id } => Self::JsonDisableTask { task_id },
            OClockClientCommandArg::JsonSwitchTask { task_id } => Self::JsonSwitchTask { task_id },
            OClockClientCommandArg::JsonRetroSwitchTask { task_id, timestamp, keep_previous_task } => Self::JsonRetroSwitchTask { task_id, timestamp, keep_previous_task },
            OClockClientCommandArg::JsonState => Self::JsonState,
            OClockClientCommandArg::Timesheet => Self::Timesheet,
        }
    }
}
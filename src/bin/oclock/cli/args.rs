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
    pub command: OClockClientCommand,
}

#[derive(Subcommand, Debug)]
pub enum OClockClientCommand {
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

impl ToString for OClockClientCommand {
    fn to_string(&self) -> String {
        match self {
            OClockClientCommand::Exit => String::from("EXIT"),
            OClockClientCommand::PushTask { name } => format!("PUSH_TASK#{}", name),
            OClockClientCommand::DisableTask { task_id } => format!("DISABLE_TASK#{}", task_id),
            OClockClientCommand::SwitchTask { task_id } => format!("SWITCH_TASK#{}", task_id),
            OClockClientCommand::CurrentTask => String::from("CURRENT_TASK"),
            OClockClientCommand::ListTasks => String::from("LIST_TASKS"),
            OClockClientCommand::JsonPushTask { name } => format!("JSON_PUSH_TASK#{}", name),
            OClockClientCommand::JsonDisableTask { task_id } => format!("JSON_DISABLE_TASK#{}", task_id),
            OClockClientCommand::JsonSwitchTask { task_id } => format!("JSON_SWITCH_TASK#{}", task_id),
            OClockClientCommand::JsonRetroSwitchTask { task_id, timestamp, keep_previous_task } => format!("JSON_RETRO_SWITCH_TASK#{task_id}#{timestamp}#{}", if *keep_previous_task { 1 } else { 0 }),
            OClockClientCommand::JsonState => String::from("JSON_STATE"),
            OClockClientCommand::Timesheet => String::from("TIMESHEET"),
        }
    }
}
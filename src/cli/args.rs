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
    /// Launch oclock in server mode
    Server,
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
    Exit,
    PushTask {
        #[clap(long, short)]
        name: String
    },
    DisableTask {
        #[clap(long, short)]
        task_id: u64
    },
    SwitchTask {
        #[clap(long, short)]
        task_id: u64
    },
    CurrentTask,
    ListTasks,
    JsonPushTask {
        #[clap(long, short)]
        name: String
    },
    JsonDisableTask {
        #[clap(long, short)]
        task_id: u64
    },
    JsonSwitchTask {
        #[clap(long, short)]
        task_id: u64
    },
    JsonRetroSwitchTask {
        #[clap(long)]
        task_id: u64,
        #[clap(long)]
        timestamp: u64,
        #[clap(long, short)]
        keep_previous_task: bool,
    },
    JsonState,
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
            OClockClientCommand::JsonState => format!("JSON_STATE#{}", serde_json::to_string(&()).expect("Error serializing JsonState args")),
            OClockClientCommand::Timesheet => String::from("TIMESHEET"),
        }
    }
}
use std::fmt;
use inflector::cases::screamingsnakecase::to_screaming_snake_case;

#[derive(Debug)]
pub enum Commands {
    Exit,
    PushTask,
    DisableTask,
    SwitchTask,
    CurrentTask,
    ListTasks,
    JsonPushTask,
    JsonDisableTask,
    JsonSwitchTask,
    JsonRetroSwitchTask,
    JsonState,
    Timesheet,
}

impl fmt::Display for Commands {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let str_command = to_screaming_snake_case(&format!("{:?}", self));
        write!(f, "{}", str_command)
    }
}
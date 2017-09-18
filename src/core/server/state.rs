use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use oclock_sqlite::connection::DB;
use oclock_sqlite::models::{NewEvent, NewTask, Task, TimesheetEntry};
use oclock_sqlite::mappers;
use oclock_sqlite::constants::SystemEventType;

pub struct State {
    database: DB,
}

#[derive(Serialize)]
pub struct ExportedState {
    current_task: Option<Task>,
    all_tasks: Vec<Task>
}

fn initialize(database: DB) -> DB {
    let connection = database.establish_connection();

    match mappers::events::get_last_event(&connection) {
        Ok(last_event) =>
            match last_event.system_event_name {
                Some(ref sys_evt) if sys_evt == &SystemEventType::Shutdown.to_string() =>
                    debug!("Already in correct state"),
                Some(_) | None => {
                    debug!("found non shutdown event");

                    let new_ts = last_event.event_timestamp;
                    
                    let event = NewEvent {
                        event_timestamp: new_ts,
                        task_id: None,
                        system_event_name: Some(SystemEventType::Shutdown.to_string()),
                    };

                    mappers::events::push_event(&connection, &event);
                }
            },
        Err(e) => 
            debug!("Error: {:?}", e)
    }
    mappers::events::remove_all_system_events(&connection, SystemEventType::Ping.to_string());

    database
}

impl State {

    pub fn new(cfg_path: String) -> State {
        State {
            database: initialize(DB::new(format!("{}/oclock.db", cfg_path)))
        }
    }

    pub fn new_task(&self, name: String) -> Result<String, String> {

        let new_task = NewTask {
            name: name
        };

        let connection = self.database.establish_connection();

        match mappers::tasks::create_task(&connection, &new_task) {
            Ok(task_id) => Result::Ok(format!("New task id '{}'", task_id)),
            Err(err) => Result::Err(format!("Error during task insert '{}'", err)),
        }
    }

    pub fn switch_task(&self, id: u64) -> Result<String, String> {
        let unix_now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        let connection = self.database.establish_connection();

        let event = NewEvent {
            event_timestamp: unix_now as i32,
            task_id: Some(id as i32),
            system_event_name: None,
        };

        match mappers::events::push_event(&connection, &event) {
            Ok(evt_id) => Result::Ok(format!("New event id '{}'", evt_id)),
            Err(err) => Result::Err(format!("Error during task switch '{}'", err)),
        }
    }

    pub fn system_event(&self, evt: SystemEventType) -> Result<String, String> {
        let unix_now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        let connection = self.database.establish_connection();

        let event = NewEvent {
            event_timestamp: unix_now as i32,
            task_id: None,
            system_event_name: Some(evt.to_string()),
        };

        match mappers::events::push_event(&connection, &event) {
            Ok(evt_id) => Result::Ok(format!("New event id '{}'", evt_id)),
            Err(err) => Result::Err(format!("Error inserting system event '{}'", err)),
        }
    }

    pub fn ping(&self) {
        let unix_now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let connection = self.database.establish_connection();

        mappers::events::move_system_event(&connection, unix_now as i32, SystemEventType::Ping.to_string())
    }

    pub fn list_tasks(&self) -> Result<Vec<Task>, String> {
        let connection = self.database.establish_connection();
        match mappers::tasks::list_tasks(&connection) {
            Ok(v) => Ok(v),
            Err(e) => Err(format!("Error retrieving tasks list: '{}'", e))
        }
    }

    pub fn full_timesheet(&self) -> Result<Vec<TimesheetEntry>, String> {
        let connection = self.database.establish_connection();
        match mappers::timesheet::full_timesheet(&connection) {
            Ok(v) => Ok(v),
            Err(e) => Err(format!("Error generating timesheet: '{}'", e))
        }
    }

    pub fn change_task_enabled_flag(&self, id: u64, enabled: bool) -> Result<String, String> {
        let connection = self.database.establish_connection();
        match mappers::tasks::change_enabled(&connection, id as i32, enabled) {
            Ok(_) => Ok(format!("Task {} enabled: {}", id, enabled)),
            Err(e) => Err(format!("Error switching task enabled flag: '{}'", e))
        }
    }

    pub fn get_current_task(&self) -> Result<Option<Task>, String> {
        let connection = self.database.establish_connection();
        match mappers::events::current_task(&connection) {
            Ok(t) => Ok(t),
            Err(e) => Err(format!("Error while fetching last task switch '{}'", e))
        }
    }

    pub fn get_state(&self) -> Result<ExportedState, String> {
        Ok(ExportedState {
            current_task: self.get_current_task()?,
            all_tasks: self.list_tasks()?
        })
    }

    pub fn retro_switch_task(&self, task_id: i32, timestamp: i32, keep_prev_task: bool) -> Result<String, String> {
        let opt_prev_task =
        match keep_prev_task {
            true => self.get_current_task()?,
            false => None
        };

        let unix_now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        let connection = self.database.establish_connection();

        let event = NewEvent {
            event_timestamp: timestamp,
            task_id: Some(task_id),
            system_event_name: None,
        };

        match mappers::events::push_event(&connection, &event) {
            Ok(evt_id) => Result::Ok(format!("New event id '{}'", evt_id)),
            Err(err) => Result::Err(format!("Error during task switch '{}'", err)),
        }?;

        match opt_prev_task {
            Some(prev_task) => {
                let redo_prev_task_evt = NewEvent {
                    event_timestamp: unix_now as i32,
                    task_id: Some(prev_task.id),
                    system_event_name: None
                };

                match mappers::events::push_event(&connection, &redo_prev_task_evt) {
                    Ok(evt_id) => Result::Ok(format!("New event id '{}'", evt_id)),
                    Err(err) => Result::Err(format!("Error during task switch '{}'", err)),
                }
            },
            None => Ok("OK".to_string())
        }
    }
}
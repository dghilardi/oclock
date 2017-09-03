use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use std::fmt;

use oclock_sqlite::connection::DB;
use oclock_sqlite::schema;
use oclock_sqlite::models::{NewEvent, NewTask};
use oclock_sqlite::mappers;

#[derive(Debug)]
pub struct Task {
    pub id: u64,
    pub name: String,
}

#[derive(Debug)]
pub enum SystemEventType {
    Startup,
    Shutdown,
    Ping,
}

impl fmt::Display for SystemEventType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub enum EventType {
    SystemEvent(SystemEventType),
    TaskSwitch(u64),
}

pub struct Event {
    pub event_type: EventType,
    pub timestamp: SystemTime,
}

pub struct State {
    pub database: DB,
    pub tasks: Vec<Task>,
    pub history: Vec<Event>,
}

impl State {
    pub fn new() -> State {
        State {
            database: DB::new("oclock.db".to_string()),
            tasks: Vec::new(),
            history: Vec::new(),
        }
    }

    pub fn new_task(&mut self, name: String) {
        use self::schema::tasks;

        let new_task = NewTask {
            name: name
        };

        let connection = self.database.establish_connection();

        mappers::tasks::create_task(&connection, &new_task);
    }

    pub fn switch_task(&mut self, id: u64) -> Result<String, String> {
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

    pub fn system_event(&mut self, evt: SystemEventType) -> Result<String, String> {
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
}
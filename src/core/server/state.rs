use std::time::SystemTime;

#[derive(Debug)]
pub struct Task {
    pub id: u64,
    pub name: String,
}

pub enum SystemEventType {
    Startup,
    Shutdown,
    Ping,
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
    pub tasks: Vec<Task>,
    pub history: Vec<Event>,
}

impl State {
    pub fn new() -> State {
        State {
            tasks: Vec::new(),
            history: Vec::new(),
        }
    }

    pub fn new_task(&mut self, name: String) {
        let new_task_id = self.tasks.len() as u64;
        self.tasks.push(Task{id: new_task_id, name: name});
        println!("{:?}", self.tasks);
    }

    pub fn switch_task(&mut self, id: u64) -> Result<String, String> {
        match self.tasks.iter().find(|&x| x.id == id) {
            Some(task) => {
                self.history.push(Event{event_type: EventType::TaskSwitch(id), timestamp: SystemTime::now()});
                Result::Ok(format!("Switched to task '{}'", task.name))
            }
            None => {
                Result::Err(format!("No task with id {}", id))
            }
        }
    }

    pub fn system_event(&mut self, evt: SystemEventType) {
        self.history.push(Event{event_type: EventType::SystemEvent(evt), timestamp: SystemTime::now()});
    }
}
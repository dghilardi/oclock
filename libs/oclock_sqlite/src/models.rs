use super::schema::*;


#[derive(Insertable)]
#[table_name="tasks"]
pub struct NewTask {
    pub name: String,
}

#[derive(Queryable)]
pub struct Task {
    pub id: i32,
    pub name: String,
}

#[derive(Insertable)]
#[table_name="events"]
pub struct NewEvent {
    pub event_timestamp: i32,
    pub task_id: Option<i32>,
    pub system_event_name: Option<String>,
}

#[derive(Queryable)]
pub struct Event {
    pub id: i32,
    pub event_timestamp: i32,
    pub task_id: Option<i32>,
    pub system_event_name: Option<String>,
}
use serde::Serialize;

use super::schema::*;

#[derive(Insertable)]
#[diesel(table_name=tasks)]
pub struct NewTask {
    pub name: String,
}

#[derive(Queryable, Serialize)]
pub struct Task {
    pub id: i32,
    pub enabled: i32,
    pub name: String,
}

#[derive(Insertable)]
#[diesel(table_name=events)]
pub struct NewEvent {
    pub event_timestamp: i32,
    pub task_id: Option<i32>,
    pub system_event_name: Option<String>,
}

#[derive(Debug, Queryable)]
pub struct Event {
    pub id: i32,
    pub event_timestamp: i32,
    pub task_id: Option<i32>,
    pub system_event_name: Option<String>,
}

#[derive(Debug, Queryable, Serialize)]
pub struct TimesheetEntry {
    pub id: i32,
    pub day: String,
    pub task_name: Option<String>,
    pub task_id: Option<i32>,
    pub system_event: Option<String>,
    pub amount: i32,
}

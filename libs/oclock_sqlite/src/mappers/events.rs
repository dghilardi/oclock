use diesel;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel::result::Error;

use models::{Event, NewEvent};

pub fn push_event(conn: &SqliteConnection, task: &NewEvent) -> Result<usize, Error> {
    use schema::events;

    diesel::insert(task)
        .into(events::table)
        .execute(conn)
}

pub fn get_last_event(conn: &SqliteConnection) -> Result<Event, Error> {
    use schema::events::dsl::*;

    events
    .order(event_timestamp.desc())
    .first(conn)
}

pub fn remove_all_system_events(conn: &SqliteConnection, event_name: String) {
    use schema::events::dsl::*;

    let num_deleted = diesel::delete(events.filter(system_event_name.eq(&event_name)))
        .execute(conn)
        .expect(&format!("Error deleting system event {}", event_name));

    debug!("deleted {} system events with type {}", num_deleted, event_name);
}

pub fn move_system_event(conn: &SqliteConnection, unix_ts: i32, event_name: String) {
    use schema::events::dsl::*;

    diesel::update(events.filter(system_event_name.eq(&event_name)))
        .set(event_timestamp.eq(unix_ts))
        .execute(conn)
        .expect(&format!("Error updating {} timestamp", event_name));
}
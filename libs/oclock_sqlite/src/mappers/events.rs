use diesel;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel::result::Error;

use crate::models::{Event, NewEvent, Task};
use crate::constants::SystemEventType;

type Backend = ::diesel::sqlite::Sqlite;

pub fn push_event(conn: &SqliteConnection, task: &NewEvent) -> Result<usize, Error> {
    use crate::schema::events;

    diesel::insert_into(events::table)
        .values(task)
        .execute(conn)
}

pub fn get_last_event(conn: &SqliteConnection) -> Result<Event, Error> {
    use crate::schema::events::dsl::*;

    events
    .order(event_timestamp.desc())
    .first(conn)
}

pub fn remove_all_system_events(conn: &SqliteConnection, event_name: String) {
    use crate::schema::events::dsl::*;

    let num_deleted = diesel::delete(events.filter(system_event_name.eq(&event_name)))
        .execute(conn)
        .expect(&format!("Error deleting system event {}", event_name));

    debug!("deleted {} system events with type {}", num_deleted, event_name);
}

pub fn move_system_event(conn: &SqliteConnection, unix_ts: i32, event_name: String) {
    use crate::schema::events::dsl::*;

    diesel::update(events.filter(system_event_name.eq(&event_name)))
        .set(event_timestamp.eq(unix_ts))
        .execute(conn)
        .expect(&format!("Error updating {} timestamp", event_name));
}

pub fn current_task(conn: &SqliteConnection) -> Result<Option<Task>, Error> {
    use crate::schema::events::dsl::*;
    use crate::schema::tasks::dsl::*;
    use crate::schema::tasks::dsl::id;

    let last_evt_query = events
    .filter(
        system_event_name.ne(SystemEventType::Ping.to_string())
        .or(system_event_name.is_null())
    )
    .order(event_timestamp.desc());

    debug!("Last event query: {}", diesel::debug_query::<Backend, _>(&last_evt_query));

    let last_evt =
    last_evt_query
    .first::<Event>(conn);

    debug!("Last event: {:?}", last_evt);

    match last_evt {
        Ok(Event{task_id: Some(curr_task_id), ..}) => {
            let task = tasks
            .filter(id.eq(curr_task_id))
            .first(conn)?;

            Ok(Some(task))
            },
        Ok(Event{task_id: None, ..}) => Ok(None),
        Err(e) => Err(e)
    }
}
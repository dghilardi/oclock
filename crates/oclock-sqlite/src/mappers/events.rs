use diesel;
use diesel::prelude::*;
use diesel::result::Error;
use diesel::sqlite::SqliteConnection;
use log::debug;

use crate::constants::SystemEventType;
use crate::models::{Event, NewEvent, Task};

type Backend = ::diesel::sqlite::Sqlite;

pub fn push_event(conn: &mut SqliteConnection, task: &NewEvent) -> Result<usize, Error> {
    use crate::schema::events;

    diesel::insert_into(events::table)
        .values(task)
        .execute(conn)
}

pub fn get_last_event(conn: &mut SqliteConnection) -> Result<Event, Error> {
    use crate::schema::events::dsl::*;

    events.order(event_timestamp.desc()).first(conn)
}

pub fn remove_all_system_events(conn: &mut SqliteConnection, event_name: String) {
    use crate::schema::events::dsl::*;

    let num_deleted = diesel::delete(events.filter(system_event_name.eq(&event_name)))
        .execute(conn)
        .expect(&format!("Error deleting system event {}", event_name));

    debug!(
        "deleted {} system events with type {}",
        num_deleted, event_name
    );
}

pub fn move_system_event(conn: &mut SqliteConnection, unix_ts: i32, event_name: String) {
    use crate::schema::events::dsl::*;

    diesel::update(events.filter(system_event_name.eq(&event_name)))
        .set(event_timestamp.eq(unix_ts))
        .execute(conn)
        .expect(&format!("Error updating {} timestamp", event_name));
}

pub fn delete_event(conn: &mut SqliteConnection, event_id: i32) -> Result<usize, Error> {
    use crate::schema::events::dsl::*;

    diesel::delete(events.filter(id.eq(event_id))).execute(conn)
}

pub fn update_event(
    conn: &mut SqliteConnection,
    event_id: i32,
    new_timestamp: Option<i32>,
    new_task_id: Option<Option<i32>>,
) -> Result<usize, Error> {
    use crate::schema::events::dsl::*;

    let target = events.filter(id.eq(event_id));

    match (new_timestamp, new_task_id) {
        (Some(ts), Some(tid)) => {
            diesel::update(target)
                .set((event_timestamp.eq(ts), task_id.eq(tid)))
                .execute(conn)
        }
        (Some(ts), None) => diesel::update(target)
            .set(event_timestamp.eq(ts))
            .execute(conn),
        (None, Some(tid)) => diesel::update(target).set(task_id.eq(tid)).execute(conn),
        (None, None) => Ok(0),
    }
}

pub fn current_task(conn: &mut SqliteConnection) -> Result<Option<Task>, Error> {
    use crate::schema::events::dsl::*;
    use crate::schema::tasks::dsl::id;
    use crate::schema::tasks::dsl::*;

    let last_evt_query = events
        .filter(
            system_event_name
                .ne(SystemEventType::Ping.to_string())
                .or(system_event_name.is_null()),
        )
        .order(event_timestamp.desc());

    debug!(
        "Last event query: {}",
        diesel::debug_query::<Backend, _>(&last_evt_query)
    );

    let last_evt = last_evt_query.first::<Event>(conn);

    debug!("Last event: {:?}", last_evt);

    match last_evt {
        Ok(Event {
            task_id: Some(curr_task_id),
            ..
        }) => {
            let task = tasks.filter(id.eq(curr_task_id)).first(conn)?;

            Ok(Some(task))
        }
        Ok(Event { task_id: None, .. }) => Ok(None),
        Err(e) => Err(e),
    }
}

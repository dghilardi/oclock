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
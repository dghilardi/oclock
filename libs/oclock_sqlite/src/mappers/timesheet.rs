use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel::result::Error;

use models::{TimesheetEntry};
use constants::SystemEventType;

pub fn full_timesheet(conn: &SqliteConnection) -> Result<Vec<TimesheetEntry>, Error> {
    use schema::v_timesheet::dsl::*;

    v_timesheet
    .filter(
        system_event.eq(SystemEventType::Startup.to_string())
        .or(system_event.is_null())
    )
    .order(day)
    .load(conn)
}
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel::result::Error;

use crate::models::{TimesheetEntry};
use crate::constants::SystemEventType;

pub fn full_timesheet(conn: &mut SqliteConnection) -> Result<Vec<TimesheetEntry>, Error> {
    use crate::schema::v_timesheet::dsl::*;

    v_timesheet
    .filter(
        system_event.eq(SystemEventType::Startup.to_string())
        .or(system_event.is_null())
    )
    .order(day)
    .load(conn)
}
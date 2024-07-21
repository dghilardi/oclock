use diesel::prelude::*;
use diesel::result::Error;
use diesel::sqlite::SqliteConnection;

use crate::constants::SystemEventType;
use crate::models::TimesheetEntry;

pub fn full_timesheet(conn: &mut SqliteConnection) -> Result<Vec<TimesheetEntry>, Error> {
    use crate::schema::v_timesheet::dsl::*;

    v_timesheet
        .filter(
            system_event
                .eq(SystemEventType::Startup.to_string())
                .or(system_event.is_null()),
        )
        .order(day)
        .load(conn)
}

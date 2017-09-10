use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel::result::Error;

use models::{TimesheetEntry};

pub fn full_timesheet(conn: &SqliteConnection) -> Result<Vec<TimesheetEntry>, Error> {
    use schema::v_timesheet::dsl::*;

    v_timesheet
    .order(day)
    .load(conn)
}
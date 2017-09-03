use diesel;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel::result::Error;

use models::{Task, NewTask};

pub fn create_task(conn: &SqliteConnection, task: &NewTask) -> Result<usize, Error> {
    use schema::tasks;

    diesel::insert(task)
        .into(tasks::table)
        .execute(conn)
}
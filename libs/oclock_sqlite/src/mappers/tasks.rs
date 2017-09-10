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

pub fn list_tasks(conn: &SqliteConnection) -> Result<Vec<Task>, Error> {
    use schema::tasks::dsl::*;

    tasks
    .order(id)
    .load(conn)
}
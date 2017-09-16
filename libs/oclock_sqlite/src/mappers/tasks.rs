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

pub fn change_enabled(conn: &SqliteConnection, task_id: i32, new_enabled: bool) -> Result<usize, Error> {
    use schema::tasks::dsl::*;

    diesel::update(tasks.filter(id.eq(&task_id)))
        .set(enabled.eq(if new_enabled {1} else {0}))
        .execute(conn)
}
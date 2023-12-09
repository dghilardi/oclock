use diesel;
use diesel::prelude::*;
use diesel::result::Error;
use diesel::sqlite::SqliteConnection;

use crate::models::{NewTask, Task};

pub fn create_task(conn: &mut SqliteConnection, task: &NewTask) -> Result<usize, Error> {
    use crate::schema::tasks;

    diesel::insert_into(tasks::table)
        .values(task)
        .execute(conn)
}

pub fn list_tasks(conn: &mut SqliteConnection) -> Result<Vec<Task>, Error> {
    use crate::schema::tasks::dsl::*;

    tasks
    .order(id)
    .load(conn)
}

pub fn change_enabled(conn: &mut SqliteConnection, task_id: i32, new_enabled: bool) -> Result<usize, Error> {
    use crate::schema::tasks::dsl::*;

    diesel::update(tasks.filter(id.eq(&task_id)))
        .set(enabled.eq(if new_enabled {1} else {0}))
        .execute(conn)
}
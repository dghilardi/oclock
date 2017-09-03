use diesel::Connection;
use diesel::sqlite::SqliteConnection;
use diesel::prelude::*;
use diesel::expression::sql_literal::sql;
use diesel::types::*;

embed_migrations!();

pub struct DB {
    connection_string: String
}

impl DB {

    pub fn new (connection_string: String) -> DB {
        let result = DB {
            connection_string: connection_string
        };

        let connection = result.establish_connection();
        embedded_migrations::run(&connection).unwrap();

        result
    }

    pub fn establish_connection(&self) -> SqliteConnection {
        let connection = SqliteConnection::establish(&self.connection_string)
            .expect(&format!("Error connecting to database at {}", self.connection_string));

        // Integer is a dummy placeholder. Compiling fails when passing ().
        sql::<(Integer)>("PRAGMA foreign_keys = ON")
            .execute(&connection)
            .expect("Should be able to enable foreign_keys");

        connection
    }

}
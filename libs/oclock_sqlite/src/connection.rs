use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::*;
use diesel::sqlite::SqliteConnection;
use diesel::Connection;
use diesel_migrations::EmbeddedMigrations;
use diesel_migrations::MigrationHarness;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub struct DB {
    connection_string: String,
}

impl DB {
    pub fn new(connection_string: String) -> DB {
        let result = DB {
            connection_string: connection_string,
        };

        let mut connection = result.establish_connection();
        connection.run_pending_migrations(MIGRATIONS).unwrap();

        result
    }

    pub fn establish_connection(&self) -> SqliteConnection {
        let mut connection = SqliteConnection::establish(&self.connection_string).expect(&format!(
            "Error connecting to database at {}",
            self.connection_string
        ));

        // Integer is a dummy placeholder. Compiling fails when passing ().
        sql::<Integer>("PRAGMA foreign_keys = ON")
            .execute(&mut connection)
            .expect("Should be able to enable foreign_keys");

        connection
    }
}

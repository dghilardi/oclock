
#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_codegen;

pub mod schema;
pub mod models;
pub mod mappers;

pub mod connection;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}

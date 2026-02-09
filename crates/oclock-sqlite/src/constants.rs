use std::fmt;

#[derive(Debug)]
pub enum SystemEventType {
    Startup,
    Shutdown,
    Ping,
}

impl fmt::Display for SystemEventType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

use crate::IdleDetector;
use std::time::Duration;
use x11rb::connection::Connection;
use x11rb::protocol::screensaver;
use x11rb::rust_connection::RustConnection;

pub struct X11IdleDetector {
    conn: RustConnection,
    root: u32,
}

impl X11IdleDetector {
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let (conn, screen_num) = RustConnection::connect(None)?;
        let root = conn.setup().roots[screen_num].root;
        // Verify we can actually query
        screensaver::query_info(&conn, root)?.reply()?;
        Ok(Self { conn, root })
    }
}

impl IdleDetector for X11IdleDetector {
    fn idle_time(&self) -> Option<Duration> {
        screensaver::query_info(&self.conn, self.root)
            .ok()?
            .reply()
            .ok()
            .map(|reply| Duration::from_millis(reply.ms_since_user_input as u64))
    }
}

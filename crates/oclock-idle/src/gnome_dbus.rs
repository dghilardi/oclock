use crate::IdleDetector;
use std::time::Duration;
use zbus::blocking::Connection;
use zbus::proxy;

#[proxy(
    interface = "org.gnome.Mutter.IdleMonitor",
    default_service = "org.gnome.Mutter.IdleMonitor",
    default_path = "/org/gnome/Mutter/IdleMonitor/Core"
)]
trait IdleMonitor {
    fn get_idletime(&self) -> zbus::Result<u64>;
}

pub struct GnomeIdleDetector {
    proxy: IdleMonitorProxyBlocking<'static>,
}

impl GnomeIdleDetector {
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let connection = Connection::session()?;
        let proxy = IdleMonitorProxyBlocking::new(&connection)?;
        // Verify the service is reachable
        proxy.get_idletime()?;
        Ok(Self { proxy })
    }
}

impl IdleDetector for GnomeIdleDetector {
    fn idle_time(&self) -> Option<Duration> {
        self.proxy
            .get_idletime()
            .ok()
            .map(|ms| Duration::from_millis(ms))
    }
}

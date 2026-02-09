use std::time::Duration;

#[cfg(feature = "x11")]
mod x11;

#[cfg(feature = "wayland")]
mod wayland;

#[cfg(feature = "gnome-dbus")]
mod gnome_dbus;

/// Events emitted by the idle monitor.
#[derive(Debug, Clone)]
pub enum IdleEvent {
    /// The user has been idle for longer than the configured threshold.
    IdleStarted,
    /// The user returned after being idle. Contains the total idle duration.
    UserReturned { idle_duration: Duration },
}

/// Trait for platform-specific idle detection backends.
///
/// Implementations poll the system for the current idle time.
pub trait IdleDetector: Send {
    /// Returns the time since last user input, or `None` if detection is unavailable.
    fn idle_time(&self) -> Option<Duration>;
}

/// Try to create the best available idle detector for the current session.
///
/// Selection order:
/// 1. GNOME DBus (works on both X11 and Wayland under GNOME)
/// 2. X11 (any X11 session)
/// 3. Wayland ext-idle-notify polling wrapper (wlroots, KDE, etc.)
///
/// Returns `None` if no backend is available.
pub fn detect() -> Option<Box<dyn IdleDetector>> {
    // Try GNOME DBus first — it works on both X11 and Wayland under GNOME
    #[cfg(feature = "gnome-dbus")]
    {
        match gnome_dbus::GnomeIdleDetector::new() {
            Ok(d) => {
                log::info!("Using GNOME DBus idle detector");
                return Some(Box::new(d));
            }
            Err(e) => log::debug!("GNOME DBus idle detector unavailable: {e}"),
        }
    }

    // Try X11 — check if we have a DISPLAY
    #[cfg(feature = "x11")]
    {
        match x11::X11IdleDetector::new() {
            Ok(d) => {
                log::info!("Using X11 idle detector");
                return Some(Box::new(d));
            }
            Err(e) => log::debug!("X11 idle detector unavailable: {e}"),
        }
    }

    // Try Wayland
    #[cfg(feature = "wayland")]
    {
        match wayland::WaylandIdleDetector::new() {
            Ok(d) => {
                log::info!("Using Wayland idle detector");
                return Some(Box::new(d));
            }
            Err(e) => log::debug!("Wayland idle detector unavailable: {e}"),
        }
    }

    log::warn!("No idle detection backend available");
    None
}

/// An idle monitor that wraps a detector and tracks state transitions.
///
/// Call `poll()` periodically to check for idle/return events.
pub struct IdleMonitor {
    detector: Box<dyn IdleDetector>,
    threshold: Duration,
    was_idle: bool,
}

impl IdleMonitor {
    pub fn new(detector: Box<dyn IdleDetector>, threshold: Duration) -> Self {
        Self {
            detector,
            threshold,
            was_idle: false,
        }
    }

    /// Poll for idle state changes. Returns an event if a transition occurred.
    pub fn poll(&mut self) -> Option<IdleEvent> {
        let idle_time = self.detector.idle_time()?;
        let is_idle = idle_time >= self.threshold;

        match (self.was_idle, is_idle) {
            (false, true) => {
                self.was_idle = true;
                Some(IdleEvent::IdleStarted)
            }
            (true, false) => {
                // User returned — idle_time is now small, but we know they were idle
                // The actual idle duration is approximated by the threshold + however
                // long they were idle beyond it. Since we only know the current idle_time
                // (which is small now), we report the threshold as minimum.
                self.was_idle = false;
                Some(IdleEvent::UserReturned {
                    idle_duration: self.threshold,
                })
            }
            _ => None,
        }
    }
}

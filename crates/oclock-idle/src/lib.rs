use std::time::Duration;

/// An event emitted by the idle detector.
#[derive(Debug, Clone)]
pub enum IdleEvent {
    /// The user has been idle for longer than the configured threshold.
    IdleStarted,
    /// The user returned after being idle. Contains the duration of the idle period.
    UserReturned { idle_duration: Duration },
}

/// Trait for platform-specific idle detection backends.
///
/// Implementations will be added for X11, Wayland (ext-idle-notify),
/// and GNOME (DBus IdleMonitor) in later phases.
pub trait IdleDetector: Send {
    /// Poll the current idle time. Returns `None` if detection is unavailable.
    fn idle_time(&self) -> Option<Duration>;
}

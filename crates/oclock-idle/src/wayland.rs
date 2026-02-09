use crate::IdleDetector;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use wayland_client::protocol::{wl_registry, wl_seat};
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::ext::idle_notify::v1::client::{
    ext_idle_notification_v1::{self, ExtIdleNotificationV1},
    ext_idle_notifier_v1::ExtIdleNotifierV1,
};

/// Default idle threshold used for the Wayland notification (1 second).
///
/// We use a very short threshold because ext-idle-notify-v1 is event-driven:
/// it tells us "user went idle" and "user came back" relative to a threshold.
/// By using a short threshold, we approximate a polling-style API where
/// `is_idle` flips quickly.
///
/// The actual idle threshold logic lives in `IdleMonitor`.
const PROBE_THRESHOLD_MS: u32 = 1000;

struct WaylandState {
    seat: Option<wl_seat::WlSeat>,
    notifier: Option<ExtIdleNotifierV1>,
    idle_flag: Arc<AtomicBool>,
}

impl Dispatch<wl_registry::WlRegistry, ()> for WaylandState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            match interface.as_str() {
                "wl_seat" => {
                    state.seat =
                        Some(registry.bind::<wl_seat::WlSeat, _, _>(name, version.min(1), qh, ()));
                }
                "ext_idle_notifier_v1" => {
                    state.notifier = Some(registry.bind::<ExtIdleNotifierV1, _, _>(
                        name,
                        version.min(1),
                        qh,
                        (),
                    ));
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &wl_seat::WlSeat,
        _: wl_seat::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtIdleNotifierV1, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &ExtIdleNotifierV1,
        _: <ExtIdleNotifierV1 as wayland_client::Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtIdleNotificationV1, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _: &ExtIdleNotificationV1,
        event: ext_idle_notification_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            ext_idle_notification_v1::Event::Idled => {
                state.idle_flag.store(true, Ordering::Relaxed);
            }
            ext_idle_notification_v1::Event::Resumed => {
                state.idle_flag.store(false, Ordering::Relaxed);
            }
            _ => {}
        }
    }
}

pub struct WaylandIdleDetector {
    idle_flag: Arc<AtomicBool>,
}

impl WaylandIdleDetector {
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let conn = Connection::connect_to_env()?;
        let display = conn.display();
        let mut event_queue = conn.new_event_queue();
        let qh = event_queue.handle();

        let idle_flag = Arc::new(AtomicBool::new(false));
        let mut state = WaylandState {
            seat: None,
            notifier: None,
            idle_flag: idle_flag.clone(),
        };

        display.get_registry(&qh, ());
        event_queue.roundtrip(&mut state)?;

        let notifier = state
            .notifier
            .as_ref()
            .ok_or("compositor does not support ext_idle_notifier_v1")?;
        let seat = state.seat.as_ref().ok_or("no wl_seat found")?;

        notifier.get_idle_notification(PROBE_THRESHOLD_MS, seat, &qh, ());
        event_queue.roundtrip(&mut state)?;

        // Spawn a background thread to pump the Wayland event queue
        std::thread::Builder::new()
            .name("oclock-idle-wayland".into())
            .spawn(move || loop {
                if event_queue.blocking_dispatch(&mut state).is_err() {
                    log::warn!("Wayland idle event queue disconnected");
                    break;
                }
            })
            .expect("failed to spawn Wayland idle thread");

        Ok(Self { idle_flag })
    }
}

impl IdleDetector for WaylandIdleDetector {
    fn idle_time(&self) -> Option<Duration> {
        // We can't know the exact idle duration on Wayland.
        // Return a large value when idle (above any reasonable threshold),
        // and zero when active. The IdleMonitor handles state transitions.
        if self.idle_flag.load(Ordering::Relaxed) {
            Some(Duration::from_secs(3600))
        } else {
            Some(Duration::ZERO)
        }
    }
}

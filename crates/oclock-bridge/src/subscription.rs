use oclock::dto::state::ExportedState;
use tokio::sync::mpsc;

/// Events emitted by the daemon state subscription.
#[derive(Debug, Clone)]
pub enum DaemonEvent {
    /// A new state snapshot was received.
    StateUpdated(ExportedState),
    /// The connection to the daemon was lost.
    Disconnected,
}

/// Start listening for daemon state updates.
///
/// Returns a tokio [`mpsc::Receiver`] that yields [`DaemonEvent`]s.
/// The background thread reads from the oclock PUB socket (via
/// `oclock::client::subscriber::subscribe`) and forwards events
/// into the async channel.
///
/// The thread exits when the receiver is dropped.
pub fn start() -> Result<mpsc::UnboundedReceiver<DaemonEvent>, oclock::client::subscriber::SubscribeError> {
    let std_rx = oclock::client::subscriber::subscribe()?;
    let (tx, rx) = mpsc::unbounded_channel();

    std::thread::Builder::new()
        .name("oclock-bridge-sub".into())
        .spawn(move || {
            for state in std_rx {
                if tx.send(DaemonEvent::StateUpdated(state)).is_err() {
                    log::debug!("Bridge subscription receiver dropped, exiting");
                    break;
                }
            }
            // If the std_rx iterator ends, the PUB socket was closed.
            let _ = tx.send(DaemonEvent::Disconnected);
        })
        .expect("failed to spawn bridge subscriber thread");

    Ok(rx)
}

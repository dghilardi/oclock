use std::sync::mpsc;

use nng::options::Options;
use nng::{Protocol, Socket};

use crate::dto::state::ExportedState;

/// Error returned when subscribing to daemon state updates fails.
#[derive(Debug, thiserror::Error)]
pub enum SubscribeError {
    #[error("Failed to create SUB socket: {0}")]
    SocketCreate(nng::Error),
    #[error("Failed to connect to PUB socket: {0}")]
    Connect(nng::Error),
    #[error("Failed to set socket option: {0}")]
    SetOption(nng::Error),
}

/// Subscribe to real-time state updates from the daemon.
///
/// Returns a [`mpsc::Receiver`] that yields an [`ExportedState`] every time
/// the daemon processes a mutation (task switch, creation, etc.).
///
/// A background thread is spawned to read from the NNG PUB socket.
/// The thread exits when the receiver is dropped.
pub fn subscribe() -> Result<mpsc::Receiver<ExportedState>, SubscribeError> {
    let socket = Socket::new(Protocol::Sub0).map_err(SubscribeError::SocketCreate)?;

    socket
        .set_opt::<nng::options::protocol::pubsub::Subscribe>(vec![])
        .map_err(SubscribeError::SetOption)?;

    socket
        .dial(crate::core::constants::SERVER_SUB_URL)
        .map_err(SubscribeError::Connect)?;

    let (tx, rx) = mpsc::channel();

    std::thread::Builder::new()
        .name("oclock-sub".into())
        .spawn(move || loop {
            match socket.recv() {
                Ok(msg) => match serde_json::from_slice::<ExportedState>(&msg) {
                    Ok(state) => {
                        if tx.send(state).is_err() {
                            log::debug!("Subscriber receiver dropped, exiting");
                            break;
                        }
                    }
                    Err(err) => {
                        log::warn!("Failed to deserialize state update: {err}");
                    }
                },
                Err(err) => {
                    log::error!("PUB socket recv error: {err}");
                    break;
                }
            }
        })
        .expect("failed to spawn subscriber thread");

    Ok(rx)
}

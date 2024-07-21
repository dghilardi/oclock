use nng::{Protocol, Socket};
use serde::de::DeserializeOwned;
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SrvInvocationError {
    #[error("Server error - {0}")]
    ServerError(String),
    #[error("Communication Error - {0}")]
    CommunicationError(String),
}
pub fn invoke_server<Req, Rep>(req: Req) -> Result<Rep, SrvInvocationError>
where
    Req: Serialize,
    Rep: DeserializeOwned,
{
    let socket = Socket::new(Protocol::Req0).map_err(|err| {
        SrvInvocationError::CommunicationError(format!("Error creating the socket - {err}"))
    })?;

    socket
        .dial(crate::core::constants::SERVER_URL)
        .map_err(|err| {
            SrvInvocationError::CommunicationError(format!(
                "Error connecting to the socket - {err}"
            ))
        })?;

    let serialized_req = serde_json::to_vec(&req).map_err(|err| {
        SrvInvocationError::CommunicationError(format!("Cannot serialize command - {err}"))
    })?;

    socket.send(&serialized_req).map_err(|(_, err)| {
        SrvInvocationError::CommunicationError(format!("Cannot send request - {err}"))
    })?;

    let out = match socket.recv() {
        Ok(reply) if reply.starts_with(b"OK#") => {
            let msg = std::str::from_utf8(&reply[3..]).map_err(|err| {
                SrvInvocationError::CommunicationError(format!("Malformed reply String - {err}"))
            })?;

            let res = serde_json::from_str::<Rep>(&msg).map_err(|err| {
                SrvInvocationError::CommunicationError(format!("Cannot deserialize json - {err}"))
            });

            if let Err(err) = &res {
                log::error!("Error deserializing '{msg}' - {err}");
            }
            res
        }
        Ok(reply) if reply.starts_with(b"ERR#") => {
            log::debug!("Recv '{:?}'.", reply);

            let msg = std::str::from_utf8(&reply[4..]).map_err(|err| {
                SrvInvocationError::CommunicationError(format!("Malformed reply String - {err}"))
            })?;

            Err(SrvInvocationError::ServerError(String::from(msg)))
        }
        Ok(reply) => {
            log::error!("not recognized response {:?}", reply);
            Err(SrvInvocationError::CommunicationError(String::from(
                "Missing reply prefix",
            )))
        }
        Err(err) => {
            log::error!("Client failed to receive reply '{}'.", err);
            Err(SrvInvocationError::CommunicationError(format!(
                "Reply was not received - {err}"
            )))
        }
    };

    socket.close();

    out
}

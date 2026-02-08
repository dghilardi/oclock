use serde::de::DeserializeOwned;
use serde::Serialize;
use thiserror::Error;
use zenoh::{Config, Wait};
use crate::core::constants::SERVER_REQ_URL;

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
    let config = Config::default();
    let session = zenoh::open(config).wait().map_err(|err| {
        SrvInvocationError::CommunicationError(format!("Error creating zenoh session - {err}"))
    })?;

    let serialized_req = serde_json::to_vec(&req).map_err(|err| {
        SrvInvocationError::CommunicationError(format!("Cannot serialize command - {err}"))
    })?;

    let receiver = session
        .get(SERVER_REQ_URL)
        .payload(serialized_req)
        .wait()
        .map_err(|err| {
            SrvInvocationError::CommunicationError(format!("Cannot send request - {err}"))
        })?;

    match receiver.recv() {
        Ok(reply) => {
            match reply.result() {
                Ok(sample) => {
                    let payload = sample.payload().to_bytes();
                    if payload.starts_with(b"OK#") {
                        let msg = std::str::from_utf8(&payload[3..]).map_err(|err| {
                            SrvInvocationError::CommunicationError(format!("Malformed reply String - {err}"))
                        })?;

                        let res = serde_json::from_str::<Rep>(&msg).map_err(|err| {
                            SrvInvocationError::CommunicationError(format!("Cannot deserialize json - {err}"))
                        });

                        if let Err(err) = &res {
                            log::error!("Error deserializing '{msg}' - {err}");
                        }
                        res
                    } else if payload.starts_with(b"ERR#") {
                        log::debug!("Recv '{:?}'.", payload);

                        let msg = std::str::from_utf8(&payload[4..]).map_err(|err| {
                            SrvInvocationError::CommunicationError(format!("Malformed reply String - {err}"))
                        })?;

                        Err(SrvInvocationError::ServerError(String::from(msg)))
                    } else {
                        log::error!("not recognized response {:?}", payload);
                        Err(SrvInvocationError::CommunicationError(String::from(
                            "Missing reply prefix",
                        )))
                    }
                }
                Err(val) => {
                    Err(SrvInvocationError::CommunicationError(format!("Value error: {:?}", val)))
                }
            }
        }
        Err(err) => {
            log::error!("Client failed to receive reply '{}'.", err);
            Err(SrvInvocationError::CommunicationError(format!(
                "Reply was not received - {err}"
            )))
        }
    }
}

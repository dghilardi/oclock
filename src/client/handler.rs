use nng::{Protocol, Socket};
use crate::dto::command::OClockClientCommand;

pub fn send_command(command: OClockClientCommand) -> bool {
    let socket = Socket::new(Protocol::Req0).unwrap();
    socket.dial(crate::core::constants::SERVER_URL).unwrap();

    let mut error_status = false;

    match socket.send(&serde_json::to_vec(&command).expect("Cannot serialize command")) {
        Ok(..) => log::debug!("Send '{:?}'.", command),
        Err(err) => log::error!("Client failed to send request '{:?}'.", err)
    }

    match socket.recv() {
        Ok(reply) if reply.starts_with(b"OK#") => {
            log::debug!("Recv '{:?}'.", reply);

            let msg = std::str::from_utf8(&reply)
                .expect("Error deserializing response")
                .replace("OK#","");

            println!("{}", msg);
        },
        Ok(reply) if reply.starts_with(b"ERR#") => {
            log::debug!("Recv '{:?}'.", reply);

            let msg = std::str::from_utf8(&reply)
                .expect("Error deserializing response")
                .replace("ERR#","");

            eprintln!("{}", msg);
            error_status = true;
        },
        Ok(reply) => {
            log::error!("not recognized response {:?}", reply);
            error_status = true;
        }
        Err(err) => log::error!("Client failed to receive reply '{}'.", err),
    }

    socket.close();
    error_status
}
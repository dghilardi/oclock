use clap::Parser;
use log::{debug, error};
use nng::{Protocol, Socket};
use crate::cli::args::{OClockArgs, OClockClientCommand, OClockCommand};

use crate::core::server::handlers;

mod core;
mod cli;

fn client(command: OClockClientCommand) -> bool {
    let socket = Socket::new(Protocol::Req0).unwrap();
    socket.dial(handlers::SERVER_URL).unwrap();

    let mut error_status = false;

    match socket.send(command.to_string().as_bytes()) {
        Ok(..) => debug!("Send '{:?}'.", command),
        Err(err) => error!("Client failed to send request '{:?}'.", err)
    }

    match socket.recv() {
        Ok(reply) if reply.starts_with(b"OK#") => {
            debug!("Recv '{:?}'.", reply);

            let msg = std::str::from_utf8(&reply)
                .expect("Error deserializing response")
                .replace("OK#","");

            println!("{}", msg);
        },
        Ok(reply) if reply.starts_with(b"ERR#") => {
            debug!("Recv '{:?}'.", reply);

            let msg = std::str::from_utf8(&reply)
                .expect("Error deserializing response")
                .replace("ERR#","");

            eprintln!("{}", msg);
            error_status = true;
        },
        Ok(reply) => {
            error!("not recognized response {:?}", reply);
            error_status = true;
        }
        Err(err) => error!("Client failed to receive reply '{}'.", err),
    }

    socket.close();
    error_status
}

fn main() {
    env_logger::init();
    let args: OClockArgs = OClockArgs::parse();

    let mut error_state = false;
    match args.subcommand {
        OClockCommand::Client(client_args) => {
            error_state = client(client_args.command)
        },
        OClockCommand::Server => handlers::server(),
    };

    std::process::exit(if error_state {
        1
    } else {
        0
    });
}
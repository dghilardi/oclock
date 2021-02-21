#![allow(unused_must_use)]

extern crate inflector;
extern crate getopts;

#[macro_use]
extern crate log;
extern crate env_logger;

extern crate schedule;

extern crate csv;
extern crate serde;
extern crate serde_json;
#[macro_use] extern crate serde_derive;

extern crate itertools;

extern crate oclock_sqlite;

use getopts::Options;

use nng::{Socket, Protocol};

use std::io::{Read, Write};

mod core;

use crate::core::server::server;
use std::error::Error;

fn client(request: String) -> bool {
    let mut socket = Socket::new(Protocol::Req0).unwrap();
    socket.dial(server::SERVER_URL).unwrap();

    let mut error_status = false;

    match socket.send(request.as_bytes()) {
        Ok(..) => debug!("Send '{}'.", request),
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

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

fn main() {
    env_logger::init();

    let args: Vec<_> = std::env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optopt("m", "mode", "oclock operation mode", "MODE");
    opts.optopt("c", "command", "oclock client command", "COMMAND#PARAMS");
    opts.optflag("h", "help", "print this help menu");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m }
        Err(f) => { panic!(f.to_string()) }
    };

    if matches.opt_present("h") {
        print_usage(&program, opts);
        return;
    }

    let mut error_state = false;
    match matches.opt_str("m") {
        Some(ref mode) if mode == "client" => {
            error_state =
            match matches.opt_str("c") {
                Some(command) => client(command),
                None => client(core::server::constants::Commands::ListTasks.to_string())
            }
        },
        Some(ref mode) if mode == "server" => server::server(),
        Some(mode) =>
            println!("mode {}", mode),
        None =>
            print_usage(&program, opts),
    };

    std::process::exit(if error_state {
        1
    } else {
        0
    });
}
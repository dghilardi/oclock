#![allow(unused_must_use)]

extern crate inflector;
extern crate nanomsg;
extern crate getopts;

#[macro_use]
extern crate log;
extern crate env_logger;

extern crate schedule;

extern crate csv;
extern crate serde;

extern crate oclock_sqlite;

use getopts::Options;

use nanomsg::{Socket, Protocol};

use std::io::{Read, Write};

mod core;

use core::server::server;

fn client(request: String) {
    let mut socket = Socket::new(Protocol::Req).unwrap();
    let mut endpoint = socket.connect(server::SERVER_URL).unwrap();

    let mut reply = String::new();

    match socket.write_all(request.as_bytes()) {
        Ok(..) => debug!("Send '{}'.", request),
        Err(err) => error!("Client failed to send request '{}'.", err)
    }

    match socket.read_to_string(&mut reply) {
        Ok(_) if reply.starts_with("OK#") => {
            debug!("Recv '{}'.", reply);

            let msg = reply.replace("OK#","");

            println!("{}", msg);
            reply.clear()
        },
        Ok(_) if reply.starts_with("ERR#") => {
            debug!("Recv '{}'.", reply);

            let msg = reply.replace("ERR#","");

            println!("{}", msg);
            reply.clear()
        },
        Ok(_) => {
            error!("not recognized response {}", reply);
            reply.clear()
        }
        Err(err) => error!("Client failed to receive reply '{}'.", err),
    }

    endpoint.shutdown();
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

    match matches.opt_str("m") {
        Some(ref mode) if mode == "client" => 
            match matches.opt_str("c") {
                Some(command) => client(command),
                None => client(core::server::constants::Commands::ListTasks.to_string())
            },
        Some(ref mode) if mode == "server" => server::server(),
        Some(mode) =>
            println!("mode {}", mode),
        None =>
            print_usage(&program, opts),
    };
}
#![allow(unused_must_use)]

extern crate nanomsg;
extern crate getopts;

use getopts::Options;

use nanomsg::{Socket, Protocol};

use std::thread;
use std::time::Duration;

use std::io::{Read, Write};

mod core;

use core::server::server;

fn client() {
    let mut socket = Socket::new(Protocol::Req).unwrap();
    let mut endpoint = socket.connect(server::SERVER_URL).unwrap();
    let mut count = 1u32;

    let mut reply = String::new();

    loop {
        let request = format!("Request #{}", count);

        match socket.write_all(request.as_bytes()) {
            Ok(..) => println!("Send '{}'.", request),
            Err(err) => {
                println!("Client failed to send request '{}'.", err);
                break
            }
        }

        match socket.read_to_string(&mut reply) {
            Ok(_) => {
                println!("Recv '{}'.", reply);
                reply.clear()
            },
            Err(err) => {
                println!("Client failed to receive reply '{}'.", err);
                break
            }
        }
        thread::sleep(Duration::from_millis(100));
        count += 1;
    }

    endpoint.shutdown();
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optopt("m", "mode", "oclock operation mode", "MODE");
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
        Some(ref mode) if mode == "client" => client(),
        Some(ref mode) if mode == "server" => server::server(),
        Some(mode) =>
            println!("mode {}", mode),
        None =>
            print_usage(&program, opts),
    };
}
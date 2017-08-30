#![allow(unused_must_use)]

extern crate nanomsg;

use nanomsg::{Socket, Protocol};

use std::thread;
use std::time::Duration;

use std::io::{Read, Write};

const SERVER_URL: &'static str = "ipc:///tmp/time-monitor.ipc";

fn client() {
    let mut socket = Socket::new(Protocol::Req).unwrap();
    let mut endpoint = socket.connect(SERVER_URL).unwrap();
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

fn server() {
    let mut socket = Socket::new(Protocol::Rep).unwrap();
    let mut endpoint = socket.bind(SERVER_URL).unwrap();
    let mut count = 1u32;

    let mut request = String::new();

    println!("Server is ready.");

    loop {

        match socket.read_to_string(&mut request) {
            Ok(_) => {
                println!("Recv '{}'.", request);

                let reply = format!("{} -> Reply #{}", request, count);
                match socket.write_all(reply.as_bytes()) {
                    Ok(..) => println!("Sent '{}'.", reply),
                    Err(err) => {
                        println!("Server failed to send reply '{}'.", err);
                        break
                    }
                }
                request.clear();
                thread::sleep(Duration::from_millis(400));
                count += 1;
            },
            Err(err) => {
                println!("Server failed to receive request '{}'.", err);
                break
            }
        }
    }

    endpoint.shutdown();
}

fn usage() {
    println!("Usage: reqrep [client|server|device]");
    println!("  Try running several clients and servers");
    println!("  And also try killing and restarting them");
    println!("  Don't forget to start the device !");
}

fn main() {
    let args: Vec<_> = std::env::args().collect();

    if args.len() < 2 {
        return usage()
    }

    match args[1].as_ref() {
        "client" => client(),
        "server" => server(),
        _ => usage()
    }
}
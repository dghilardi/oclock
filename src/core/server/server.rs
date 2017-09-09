use nanomsg::{Socket, Protocol, Error};

use std::str;
use std::thread;
use std::sync::mpsc;
use std::time::Duration;

use std::io::{Write};

use core::server::state::{State, SystemEventType};

pub const SERVER_URL: &'static str = "ipc:///tmp/time-monitor.ipc";

pub const PREFIX_PUSH_TASK: &'static str = "PUSH_TASK#";
pub const PREFIX_SWITCH_TASK: &'static str = "SWITCH_TASK#";

enum Command {
    Exit,
    PushTask(String),
    SwitchTask(u64),
}

enum ThreadCmd {
    Stop
}

fn nanomsg_listen(cmd_sender: mpsc::Sender<Command>, thread_cmd_receiver: mpsc::Receiver<ThreadCmd>) {
    let mut socket = Socket::new(Protocol::Rep).unwrap();
    let mut endpoint = socket.bind(SERVER_URL).unwrap();

    let mut buffer = Vec::new();

    println!("Server is ready.");

    loop {

        match socket.nb_read_to_end(&mut buffer) {
            Ok(_) => {
                let reply =
                match str::from_utf8(buffer.as_slice()) {
                    Ok(msg) if msg == "EXIT" => {
                        cmd_sender.send(Command::Exit);
                        format!("OK")
                    },
                    Ok(msg) if msg.starts_with(PREFIX_PUSH_TASK) => {
                        cmd_sender.send(Command::PushTask(msg.replace(PREFIX_PUSH_TASK, "")));
                        format!("OK")
                    },
                    Ok(msg) if msg.starts_with(PREFIX_SWITCH_TASK) && msg.replace(PREFIX_SWITCH_TASK, "").parse::<u64>().is_ok() => {
                        cmd_sender.send(Command::SwitchTask(msg.replace(PREFIX_SWITCH_TASK, "").parse::<u64>().unwrap()));
                        format!("OK")
                    },
                    Ok(msg) => {
                        println!("message '{}' not handled", msg);
                        cmd_sender.send(Command::Exit);
                        format!("ERROR#Not recognized")
                    },
                    Err(e) => {
                        panic!("Invalid UTF-8 sequence: {}", e)
                    },
                };

                match socket.write_all(reply.as_bytes()) {
                    Ok(..) => println!("Sent '{}'.", reply),
                    Err(err) => {
                        println!("Server failed to send reply '{}'.", err);
                        break
                    }
                }
                buffer.clear();
            },
            Err(Error::TryAgain) => {
                println!("No message received");
            },
            Err(err) => {
                println!("Server failed to receive request '{}'.", err);
                break
            }
        }

        if let Ok(thread_cmd) = thread_cmd_receiver.try_recv() {
            match thread_cmd {
                ThreadCmd::Stop => break
            }
        }
        
        thread::sleep(Duration::from_millis(400));
    }

    endpoint.shutdown();
}

pub fn server() {
    let mut state = State::new();
    state.system_event(SystemEventType::Startup);
    state.system_event(SystemEventType::Ping);

    let (cmd_sender, cmd_receiver) = mpsc::channel();
    let (thread_cmd_sender, thread_cmd_receiver) = mpsc::channel();

    let nanomsg_sender = cmd_sender.clone();
    let cmd_listener_thread = thread::spawn(move || {
        nanomsg_listen(nanomsg_sender, thread_cmd_receiver);
    });

    loop {
        match cmd_receiver.recv() {
            Ok(Command::Exit) => break,
            Ok(Command::PushTask(new_task_name)) => state.new_task(new_task_name),
            Ok(Command::SwitchTask(task_id)) => { state.switch_task(task_id); },
            _ => {}
        }
    }

    thread_cmd_sender.send(ThreadCmd::Stop);
    cmd_listener_thread.join();

    state.system_event(SystemEventType::Shutdown);
}
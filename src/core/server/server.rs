use nanomsg::{Socket, Protocol, Error};

use std::str;
use std::thread;
use std::time::Duration;

use schedule::{Agenda, Job};

use std::io::{Write};

use core::server::state::{State, SystemEventType};

pub const SERVER_URL: &'static str = "ipc:///tmp/time-monitor.ipc";

pub const PREFIX_PUSH_TASK: &'static str = "PUSH_TASK#";
pub const PREFIX_SWITCH_TASK: &'static str = "SWITCH_TASK#";

enum MsgListenerStatus {
    Continue,
    Terminate,
    Fail
}

fn nanomsg_listen(socket: &mut Socket, state: &State) -> MsgListenerStatus {
    let mut buffer = Vec::new();

    match socket.nb_read_to_end(&mut buffer) {
        Ok(_) => {
            let status = match str::from_utf8(buffer.as_slice()) {
                Ok(msg) if msg == "EXIT" => MsgListenerStatus::Terminate,
                Ok(_) => MsgListenerStatus::Continue,
                Err(_) => MsgListenerStatus::Fail,
            };

            let cmd_outcome =
            match str::from_utf8(buffer.as_slice()) {
                Ok(msg) if msg == "EXIT" => {
                    Ok(format!("bye bye..."))
                },
                Ok(msg) if msg.starts_with(PREFIX_PUSH_TASK) => {
                    state.new_task(msg.replace(PREFIX_PUSH_TASK, ""))
                },
                Ok(msg) if msg.starts_with(PREFIX_SWITCH_TASK) && msg.replace(PREFIX_SWITCH_TASK, "").parse::<u64>().is_ok() => {
                    let task_id = msg.replace(PREFIX_SWITCH_TASK, "").parse::<u64>().unwrap();
                    state.switch_task(task_id)
                },
                Ok(msg) => {
                    error!("message '{}' not handled", msg);
                    Err(format!("Not recognized"))
                },
                Err(e) => {
                    error!("Invalid UTF-8 sequence: {}", e);
                    Err(format!("Invalid UTF-8 sequence"))
                },
            };

            let reply =
            match cmd_outcome {
                Ok(msg) => format!("OK#{}", msg),
                Err(msg) => format!("ERR#{}", msg),
            };

            match socket.write_all(reply.as_bytes()) {
                Ok(..) => println!("Sent '{}'.", reply),
                Err(err) => {
                    error!("Server failed to send reply '{}'.", err)
                }
            };

            buffer.clear();

            status
        },
        Err(Error::TryAgain) => {
            debug!("No message received");
            MsgListenerStatus::Continue
        },
        Err(err) => {
            error!("Server failed to receive request '{}'.", err);
            MsgListenerStatus::Continue
        }
    }

}

pub fn server() {
    let mut nanomsg_socket = Socket::new(Protocol::Rep).unwrap();
    let mut nanomsg_endpoint = nanomsg_socket.bind(SERVER_URL).unwrap();

    let state = State::new();
    state.system_event(SystemEventType::Startup);
    state.system_event(SystemEventType::Ping);

    let mut daemon_status = MsgListenerStatus::Continue;
    let mut a = Agenda::new();

    // Run every second
    a.add(Job::new(|| {
        daemon_status = nanomsg_listen(&mut nanomsg_socket, &state);
    }, "* * * * * *".parse().unwrap()));

    // Run every minute
    a.add(Job::new(|| {
        state.ping();
    }, "0 * * * * *".parse().unwrap()));

    // Check and run pending jobs in agenda every 500 milliseconds
    loop {
        a.run_pending();

        match daemon_status {
            MsgListenerStatus::Continue => (),
            _ => break,
        }

        thread::sleep(Duration::from_millis(300));
    }

    state.system_event(SystemEventType::Shutdown);
    nanomsg_endpoint.shutdown();
}
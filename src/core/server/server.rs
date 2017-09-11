use nanomsg;
use nanomsg::{Socket, Protocol};

use std::str;
use std::env;
use std::fs;
use std::thread;
use std::time::Duration;
use std::error::Error;

use schedule::{Agenda, Job};

use std::io::{Write};
use csv::Writer;
use serde;

use core::server::state::{State, SystemEventType};
use core::server::constants::Commands;

pub const SERVER_URL: &'static str = "ipc:///tmp/time-monitor.ipc";

pub const SEP: &'static str = "#";

enum MsgListenerStatus {
    Continue,
    Terminate,
    Fail
}

fn vec_to_csv<T>(items: Vec<T>) -> Result<String, Box<Error>> where
    T: serde::ser::Serialize
{
    let mut wtr = Writer::from_writer(vec![]);
    for item in items {
        wtr.serialize(item);
    }

    let data = String::from_utf8(wtr.into_inner()?)?;
    Ok(data)
}

fn handle_msg(msg: &str, state: &State) -> Result<String, String> {
    let splitted_cmd: Vec<&str> = msg.split(SEP).collect();
    let (command, args) = splitted_cmd.split_at(1);
    match command.first() {
        Some(m) if m == &Commands::Exit.to_string() => Ok(format!("bye bye...")),
        Some(m) if m == &Commands::ListTasks.to_string() => {
            let tasks = state.list_tasks()?;
            match vec_to_csv(tasks) {
                Ok(csv) => Ok(csv),
                Err(e) => Err(format!("Error generating csv '{}'", e))
            }
        },
        Some(m) if m == &Commands::Timesheet.to_string() => {
            let timesheet = state.full_timesheet()?;
            match vec_to_csv(timesheet) {
                Ok(csv) => Ok(csv),
                Err(e) => Err(format!("Error generating csv '{}'", e)),
            }
        },
        Some(m) if m == &Commands::PushTask.to_string() => state.new_task(args.join(SEP)),
        Some(m) if m == &Commands::SwitchTask.to_string() => {
            let task_id = args.join(SEP).parse::<u64>().unwrap();
            state.switch_task(task_id)
        },
        Some(no_match) => {
            error!("message '{:?}' not handled", no_match);
            Err(format!("Not recognized"))
        },
        None => {
            error!("command not recognized");
            Err(format!("Not recognized"))
        }
    }
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
                Ok(msg) => handle_msg(msg, state),
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
        Err(nanomsg::Error::TryAgain) => {
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

    let cfg_path = 
    match env::var("HOME") {
        Ok(path) => format!("{}/.oclock", path),
        Err(_) => ".".to_string()
    };

    fs::create_dir_all(&cfg_path).unwrap_or_else(|why| {
        println!("! {:?}", why.kind());
    });

    let state = State::new(cfg_path);
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
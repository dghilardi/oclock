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
use serde_json;

use core::server::state::{State, TimesheetPivotRecord};
use core::server::constants::Commands;
use oclock_sqlite::constants::SystemEventType;

extern crate ctrlc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

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

fn timesheet_to_csv(tasks: Vec<String>, records: Vec<TimesheetPivotRecord>) -> Result<String, Box<Error>> {
    let mut wtr = Writer::from_writer(vec![]);
    wtr.serialize(("day", tasks));
    for item in records {
        let entries_str: Vec<String> = item.entries.iter()
            .map(|e| 
                format!("{:02}:{:02}:{:02}", e/3600, (e-(e/3600)*3600)/60, e-(e/60)*60)
            )
            .collect();
        wtr.serialize((item.day, entries_str));
    }

    let data = String::from_utf8(wtr.into_inner()?)?;
    Ok(data)
}

fn compute_state(state: &State) -> Result<String, String> {
    let exp_state = state.get_state()?;
    match serde_json::to_string(&exp_state) {
        Ok(json) => Ok(json),
        Err(e) => Err(format!("Error serializing state {}", e))
    }
}

fn handle_msg(msg: &str, state: &State) -> Result<String, String> {
    let splitted_cmd: Vec<&str> = msg.split(SEP).collect();
    let (command, args) = splitted_cmd.split_at(1);
    match command.first() {
        Some(m) if m == &Commands::Exit.to_string() => Ok(format!("bye bye...")),
        Some(m) if m == &Commands::CurrentTask.to_string() => {
            let task = state.get_current_task()?;
            match task {
                Some(t) => Ok(t.name),
                None => Ok("None".to_string())
            }
        },
        Some(m) if m == &Commands::ListTasks.to_string() => {
            let tasks = state.list_tasks()?;
            match vec_to_csv(tasks) {
                Ok(csv) => Ok(csv),
                Err(e) => Err(format!("Error generating csv '{}'", e))
            }
        },
        Some(m) if m == &Commands::Timesheet.to_string() => {
            let (tasks, timesheet) = state.full_timesheet()?;

            match timesheet_to_csv(tasks, timesheet) {
                Ok(csv) => Ok(csv),
                Err(e) => Err(format!("Error generating csv '{}'", e)),
            }
        },
        Some(m) if m == &Commands::PushTask.to_string() => state.new_task(args.join(SEP)),
        Some(m) if m == &Commands::DisableTask.to_string() => {
            let task_id = args.join(SEP).parse::<u64>().unwrap();
            state.change_task_enabled_flag(task_id, false)            
        },
        Some(m) if m == &Commands::SwitchTask.to_string() => {
            let task_id = args.join(SEP).parse::<u64>().unwrap();
            state.switch_task(task_id)
        },
        Some(m) if m == &Commands::JsonPushTask.to_string() => {
            state.new_task(args.join(SEP))?;
            compute_state(state)
        },
        Some(m) if m == &Commands::JsonDisableTask.to_string() => {
            let task_id = args.join(SEP).parse::<u64>().unwrap();
            state.change_task_enabled_flag(task_id, false);
            compute_state(state)
        },
        Some(m) if m == &Commands::JsonSwitchTask.to_string() => {
            let task_id = args.join(SEP).parse::<u64>().unwrap();
            state.switch_task(task_id)?;
            compute_state(state)
        },
        Some(m) if m == &Commands::JsonRetroSwitchTask.to_string() => { 
            let task_id = 
            match args.get(0) {
                Some(task_id_str) => Ok(task_id_str.parse::<u64>().unwrap()),
                None => Err("No task_id parameter found".to_string())
            }?;
            let timestamp = 
            match args.get(1) {
                Some(task_id_str) => Ok(task_id_str.parse::<u64>().unwrap()),
                None => Err("No task_id parameter found".to_string())
            }?;
            let keep_prev_task = 
            match args.get(2) {
                Some(task_id_str) => Ok(task_id_str.parse::<u64>().unwrap() > 0),
                None => Err("No task_id parameter found".to_string())
            }?;
            state.retro_switch_task(task_id as i32, timestamp as i32, keep_prev_task)?;
            compute_state(state)
        },
        Some(m) if m == &Commands::JsonState.to_string() => {
            compute_state(state)
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

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    // Check and run pending jobs in agenda every 500 milliseconds
    loop {
        a.run_pending();

        match daemon_status {
            MsgListenerStatus::Continue => (),
            _ => break,
        }

        if !running.load(Ordering::SeqCst) {
            break;
        }

        thread::sleep(Duration::from_millis(300));
    }

    println!("Shutting down");

    state.system_event(SystemEventType::Shutdown);
    nanomsg_endpoint.shutdown();
}
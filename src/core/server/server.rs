use nng;
use nng::{Socket, Protocol};

use log::{error, debug};

use std::str;
use std::env;
use std::fs;
use std::thread;
use std::time::Duration;
use std::error::Error;

use schedule::{Agenda, Job};

use csv::Writer;
use serde;
use serde_json;

use crate::core::server::state::{State, TimesheetPivotRecord};
use crate::core::server::constants::Commands;
use oclock_sqlite::constants::SystemEventType;

extern crate ctrlc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::sync::mpsc::{Sender, Receiver, TryRecvError};

pub const SERVER_URL: &'static str = "ipc:///tmp/time-monitor.ipc";

pub const SEP: &'static str = "#";

enum MsgListenerStatus {
    Continue,
    Terminate,
    Fail
}

fn vec_to_csv<T>(items: Vec<T>) -> Result<String, Box<dyn Error>> where
    T: serde::ser::Serialize
{
    let mut wtr = Writer::from_writer(vec![]);
    for item in items {
        let out = wtr.serialize(item);
        if let Err(err) = out {
            log::warn!("Error serializing item - {err}");
        }
    }

    let data = String::from_utf8(wtr.into_inner()?)?;
    Ok(data)
}

#[test]
fn test_time_format() {
    assert_eq!(format_time_interval(&0), "00:00:00");
    assert_eq!(format_time_interval(&1), "00:00:01");
    assert_eq!(format_time_interval(&60), "00:01:00");
    assert_eq!(format_time_interval(&3600), "01:00:00");
    
    assert_eq!(format_time_interval(&45296), "12:34:56");
}

fn format_time_interval(i: &i32) -> String {
    format!("{:02}:{:02}:{:02}", i/3600, (i-(i/3600)*3600)/60, i-(i/60)*60)
}

fn timesheet_to_csv(tasks: Vec<String>, records: Vec<TimesheetPivotRecord>) -> Result<String, Box<dyn Error>> {
    let mut wtr = Writer::from_writer(vec![]);
    let out = wtr.serialize(("day", tasks));
    if let Err(err) = out {
        log::warn!("Error serializing tasks - {err}");
    }
    for item in records {
        let entries_str: Vec<String> = item.entries.iter()
            .map(|e| 
                format_time_interval(e)
            )
            .collect();
        let out = wtr.serialize((item.day, entries_str));
        if let Err(err) = out {
            log::warn!("Error serializing day entries - {err}");
        }
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
            let out = state.change_task_enabled_flag(task_id, false);
            if let Err(err) = out {
                log::warn!("Error disabling task {task_id} - {err}");
            }
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
    match socket.recv() {
        Ok(message) => {
            let message_str = str::from_utf8(&message);
            let status = match message_str {
                Ok(msg) if msg == "EXIT" => MsgListenerStatus::Terminate,
                Ok(_) => MsgListenerStatus::Continue,
                Err(_) => MsgListenerStatus::Fail,
            };

            let cmd_outcome =
            match message_str {
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

            match socket.send(reply.as_bytes()) {
                Ok(..) => println!("Sent '{}'.", reply),
                Err(err) => {
                    error!("Server failed to send reply '{:?}'.", err)
                }
            };

            status
        },
        Err(nng::Error::TryAgain) => {
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
    let mut nanomsg_socket = Socket::new(Protocol::Rep0).unwrap();
    nanomsg_socket.listen(SERVER_URL).unwrap();
    let (command_tx, command_rx): (Sender<MsgListenerStatus>, Receiver<MsgListenerStatus>) = mpsc::channel();

    let cfg_path = 
    match env::var("HOME") {
        Ok(path) => format!("{}/.oclock", path),
        Err(_) => ".".to_string()
    };

    fs::create_dir_all(&cfg_path).unwrap_or_else(|why| {
        println!("! {:?}", why.kind());
    });

    let state = State::new(cfg_path);
    let out = state.system_event(SystemEventType::Startup);
    if let Err(err) = out {
        log::warn!("Error pushing system event startup - {err}");
    }
    let out = state.system_event(SystemEventType::Ping);
    if let Err(err) = out {
        log::warn!("Error pushing system event ping - {err}");
    }

    let mut a = Agenda::new();

    // Run every second
    a.add(Job::new(|| {
        let daemon_status = nanomsg_listen(&mut nanomsg_socket, &state);
        let out = command_tx.send(daemon_status);
        if let Err(err) = out {
            log::error!("Error sending command in channel - {err}");
        }
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

        match command_rx.try_recv() {
            Ok(MsgListenerStatus::Continue) => (),
            Err(TryRecvError::Empty) => (),
            _ => break,
        }

        if !running.load(Ordering::SeqCst) {
            break;
        }

        thread::sleep(Duration::from_millis(300));
    }

    println!("Shutting down");

    let out = state.system_event(SystemEventType::Shutdown);
    if let Err(err) = out {
        log::error!("Error writing system event shutdown - {err}");
    }
}
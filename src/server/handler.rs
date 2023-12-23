extern crate ctrlc;

use std::env;
use std::error::Error;
use std::fs;
use std::str;
use std::sync::{Arc, mpsc};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::thread;
use std::time::Duration;

use csv::Writer;
use nng;
use nng::{Protocol, Socket};
use nng::options::{Options, RecvTimeout, SendTimeout};
use oclock_sqlite::constants::SystemEventType;
use schedule::{Agenda, Job};
use serde;
use serde_json;

use crate::dto::command::OClockClientCommand;
use crate::server::state::{State, TimesheetPivotRecord};

pub const SEP: &str = "#";

enum MsgListenerStatus {
    Continue,
    Terminate,
    Fail,
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
    format!("{:02}:{:02}:{:02}", i / 3600, (i - (i / 3600) * 3600) / 60, i - (i / 60) * 60)
}

fn timesheet_to_csv(tasks: Vec<String>, records: Vec<TimesheetPivotRecord>) -> Result<String, Box<dyn Error>> {
    let mut wtr = Writer::from_writer(vec![]);
    let out = wtr.serialize(("day", tasks));
    if let Err(err) = out {
        log::warn!("Error serializing tasks - {err}");
    }
    for item in records {
        let entries_str: Vec<String> = item.entries.iter()
            .map(format_time_interval)
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

fn handle_msg(msg: OClockClientCommand, state: &State) -> Result<String, String> {
    match msg {
        OClockClientCommand::Exit => Ok(String::from("bye bye...")),
        OClockClientCommand::CurrentTask => {
            let task = state.get_current_task()?;
            match task {
                Some(t) => Ok(t.name),
                None => Ok("None".to_string())
            }
        }
        OClockClientCommand::ListTasks => {
            let tasks = state.list_tasks()?;
            match vec_to_csv(tasks) {
                Ok(csv) => Ok(csv),
                Err(e) => Err(format!("Error generating csv '{}'", e))
            }
        }
        OClockClientCommand::Timesheet => {
            let (tasks, timesheet) = state.full_timesheet()?;

            match timesheet_to_csv(tasks, timesheet) {
                Ok(csv) => Ok(csv),
                Err(e) => Err(format!("Error generating csv '{}'", e)),
            }
        }
        OClockClientCommand::PushTask { name } => state.new_task(name),
        OClockClientCommand::DisableTask { task_id } => state.change_task_enabled_flag(task_id, false),
        OClockClientCommand::SwitchTask { task_id } => state.switch_task(task_id),
        OClockClientCommand::JsonPushTask { name } => {
            state.new_task(name)?;
            compute_state(state)
        }
        OClockClientCommand::JsonDisableTask { task_id } => {
            let out = state.change_task_enabled_flag(task_id, false);
            if let Err(err) = out {
                log::warn!("Error disabling task {task_id} - {err}");
            }
            compute_state(state)
        }
        OClockClientCommand::JsonSwitchTask { task_id } => {
            state.switch_task(task_id)?;
            compute_state(state)
        }
        OClockClientCommand::JsonRetroSwitchTask { task_id, timestamp, keep_previous_task } => {
            state.retro_switch_task(task_id as i32, timestamp as i32, keep_previous_task)?;
            compute_state(state)
        }
        OClockClientCommand::JsonState => {
            compute_state(state)
        }
    }
}

fn nanomsg_listen(socket: &mut Socket, state: &State) -> MsgListenerStatus {
    match socket.recv() {
        Ok(message) => {
            let message_str = serde_json::from_slice(&message);
            let status = match message_str {
                Ok(OClockClientCommand::Exit) => MsgListenerStatus::Terminate,
                Ok(_) => MsgListenerStatus::Continue,
                Err(_) => MsgListenerStatus::Fail,
            };

            let cmd_outcome =
                match message_str {
                    Ok(msg) => handle_msg(msg, state),
                    Err(e) => {
                        log::error!("Invalid message received: {}", e);
                        Err(String::from("Invalid message"))
                    }
                };

            let reply =
                match cmd_outcome {
                    Ok(msg) => format!("OK#{}", msg),
                    Err(msg) => format!("ERR#{}", msg),
                };

            match socket.send(reply.as_bytes()) {
                Ok(..) => println!("Sent '{}'.", reply),
                Err(err) => {
                    log::error!("Server failed to send reply '{:?}'.", err)
                }
            };

            status
        }
        Err(nng::Error::TryAgain) => {
            log::debug!("No message received");
            MsgListenerStatus::Continue
        }
        Err(nng::Error::TimedOut) => {
            log::debug!("No message received");
            MsgListenerStatus::Continue
        }
        Err(err) => {
            log::error!("Server failed to receive request '{}'.", err);
            MsgListenerStatus::Continue
        }
    }
}

pub fn server() {
    let mut nanomsg_socket = Socket::new(Protocol::Rep0).unwrap();
    nanomsg_socket.set_opt::<SendTimeout>(Some(Duration::from_millis(500))).expect("Error setting SendTimeout opt");
    nanomsg_socket.set_opt::<RecvTimeout>(Some(Duration::from_millis(5000))).expect("Error setting RecvTimeout opt");

    nanomsg_socket.listen(crate::core::constants::SERVER_URL).unwrap();
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
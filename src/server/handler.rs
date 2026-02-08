use std::env;
use std::error::Error;
use std::fs;
use std::str;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

use csv::Writer;
use zenoh::{Config, Wait, Session}; // Assuming Session is exported or available via prelude if any.
// If Session is not exported directly, I might need to import it specifically or use inferred type.
// But usually zenoh::Session is available.
use oclock_sqlite::constants::SystemEventType;
use schedule::{Agenda, Job};
use serde::Serialize;
use serde::de::Error as SerdeError;
use crate::core::constants::{SERVER_SUB_URL, SERVER_REQ_URL};
use crate::dto::command::OClockClientCommand;
use crate::server::state::{State, TimesheetPivotRecord};

pub const SEP: &str = "#";

enum MsgListenerStatus {
    Continue,
    Terminate,
    Fail,
}

fn vec_to_csv<T>(items: Vec<T>) -> Result<String, Box<dyn Error>>
where
    T: serde::ser::Serialize,
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
    format!(
        "{:02}:{:02}:{:02}",
        i / 3600,
        (i - (i / 3600) * 3600) / 60,
        i - (i / 60) * 60
    )
}

fn timesheet_to_csv(
    tasks: Vec<String>,
    records: Vec<TimesheetPivotRecord>,
) -> Result<String, Box<dyn Error>> {
    let mut wtr = Writer::from_writer(vec![]);
    let out = wtr.serialize(("day", tasks));
    if let Err(err) = out {
        log::warn!("Error serializing tasks - {err}");
    }
    for item in records {
        let entries_str: Vec<String> = item.entries.iter().map(format_time_interval).collect();
        let out = wtr.serialize((item.day, entries_str));
        if let Err(err) = out {
            log::warn!("Error serializing day entries - {err}");
        }
    }

    let data = String::from_utf8(wtr.into_inner()?)?;
    Ok(data)
}

fn compute_state(state: &State) -> Result<serde_json::Value, String> {
    let exp_state = state.get_state()?;
    match serde_json::to_value(&exp_state) {
        Ok(json) => Ok(json),
        Err(e) => Err(format!("Error serializing state {}", e)),
    }
}

fn pub_state(state: &impl Serialize, session: &Session) {
    log::info!("Sending state update");
    let serialized = serde_json::to_vec(state).expect("error serializing state");
    let out = session.put(SERVER_SUB_URL, serialized).wait();
    if let Err(err) = out {
        log::error!("Error publishing state - {err:?}");
    }
}

fn handle_msg(msg: OClockClientCommand, state: &State, session: &Session) -> Result<serde_json::Value, String> {
    match msg {
        OClockClientCommand::Exit => Ok(serde_json::Value::String(String::from("bye bye..."))),
        OClockClientCommand::CurrentTask => {
            let task = state.get_current_task()?;
            match task {
                Some(t) => Ok(serde_json::Value::String(t.name)),
                None => Ok(serde_json::Value::String(String::from("None"))),
            }
        }
        OClockClientCommand::ListTasks => {
            let tasks = state.list_tasks()?;
            match vec_to_csv(tasks) {
                Ok(csv) => Ok(serde_json::Value::String(csv)),
                Err(e) => Err(format!("Error generating csv '{}'", e)),
            }
        }
        OClockClientCommand::Timesheet => {
            let (tasks, timesheet) = state.full_timesheet()?;

            match timesheet_to_csv(tasks, timesheet) {
                Ok(csv) => Ok(serde_json::Value::String(csv)),
                Err(e) => Err(format!("Error generating csv '{}'", e)),
            }
        }
        OClockClientCommand::PushTask { name } => {
            let result = state.new_task(name);
            if let Ok(state) = compute_state(state) {
                pub_state(&state, session);
            }
            result
        },
        OClockClientCommand::DisableTask { task_id } => {
            let result = state.change_task_enabled_flag(task_id, false);
            if let Ok(state) = compute_state(state) {
                pub_state(&state, session);
            }
            result
        }
        OClockClientCommand::SwitchTask { task_id } => {
            let result = state.switch_task(task_id);
            if let Ok(state) = compute_state(&state) {
                pub_state(&state, session);
            }
            result
        },
        OClockClientCommand::JsonPushTask { name } => {
            state.new_task(name)?;
            let state = compute_state(state);
            if let Ok(state) = &state {
                pub_state(state, session);
            }
            state
        }
        OClockClientCommand::JsonDisableTask { task_id } => {
            let out = state.change_task_enabled_flag(task_id, false);
            if let Err(err) = out {
                log::warn!("Error disabling task {task_id} - {err}");
            }
            let state = compute_state(state);
            if let Ok(state) = &state {
                pub_state(state, session);
            }
            state
        }
        OClockClientCommand::JsonSwitchTask { task_id } => {
            state.switch_task(task_id)?;
            let state = compute_state(state);
            if let Ok(state) = &state {
                pub_state(state, session);
            }
            state
        }
        OClockClientCommand::JsonRetroSwitchTask {
            task_id,
            timestamp,
            keep_previous_task,
        } => {
            state.retro_switch_task(task_id as i32, timestamp as i32, keep_previous_task)?;
            let state = compute_state(state);
            if let Ok(state) = &state {
                pub_state(state, session);
            }
            state
        }
        OClockClientCommand::JsonState => compute_state(state),
    }
}

pub fn server() {
    let config = Config::default();
    let session = Arc::new(zenoh::open(config).wait().unwrap());

    let (command_tx, command_rx): (Sender<MsgListenerStatus>, Receiver<MsgListenerStatus>) =
        mpsc::channel();

    let cfg_path = match env::var("HOME") {
        Ok(path) => format!("{}/.oclock", path),
        Err(_) => ".".to_string(),
    };

    fs::create_dir_all(&cfg_path).unwrap_or_else(|why| {
        println!("! {:?}", why.kind());
    });

    let state = Arc::new(State::new(cfg_path));

    // Initial system events
    let out = state.system_event(SystemEventType::Startup);
    if let Err(err) = out {
        log::warn!("Error pushing system event startup - {err}");
    }
    let out = state.system_event(SystemEventType::Ping);
    if let Err(err) = out {
        log::warn!("Error pushing system event ping - {err}");
    }

    // Spawn Zenoh Query Listener
    let session_clone = session.clone();
    let state_clone = state.clone();
    let command_tx_clone = command_tx.clone();

    thread::spawn(move || {
        let subscriber = session_clone.declare_queryable(SERVER_REQ_URL).wait().unwrap();
        while let Ok(query) = subscriber.recv() {
            // query.payload() is expected to be command
            // payload() returns Result<Payload, Error> or just Payload?
            // In Zenoh 1.0, query.payload() returns Option<Payload>? No.
            // query has payload() method or field.

            let payload = query.payload();

            let message_str: Result<OClockClientCommand, _> = match payload {
                Some(p) => serde_json::from_slice(&p.to_bytes()),
                None => Err(SerdeError::custom("Missing payload")),
            };

            let status = match message_str {
                Ok(OClockClientCommand::Exit) => MsgListenerStatus::Terminate,
                Ok(_) => MsgListenerStatus::Continue,
                Err(_) => MsgListenerStatus::Fail,
            };

            let cmd_outcome = match message_str {
                Ok(msg) => handle_msg(msg, &state_clone, &session_clone),
                Err(e) => {
                    log::error!("Invalid message received: {}", e);
                    Err(String::from("Invalid message"))
                }
            };

            let reply_str = match cmd_outcome {
                Ok(msg) => format!("OK#{}", msg),
                Err(msg) => format!("ERR#{}", msg),
            };

            // Reply to query
            match query.reply(query.key_expr(), reply_str).wait() {
                 Ok(_) => println!("Sent reply."),
                 Err(err) => log::error!("Server failed to send reply '{:?}'.", err),
            }

            // Send status to main loop if terminate
            if let MsgListenerStatus::Terminate = status {
                 let _ = command_tx_clone.send(status);
                 break;
            }
        }
    });

    let mut a = Agenda::new();

    // Run every minute (Ping)
    let state_ping = state.clone();
    a.add(Job::new(
        move || {
            state_ping.ping();
        },
        "0 * * * * *".parse().unwrap(),
    ));

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    // Check and run pending jobs in agenda every 500 milliseconds
    loop {
        a.run_pending();

        match command_rx.try_recv() {
            Ok(MsgListenerStatus::Terminate) => break,
            Ok(_) => (),
            Err(TryRecvError::Empty) => (),
            Err(TryRecvError::Disconnected) => break,
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

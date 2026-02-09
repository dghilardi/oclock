use ksni::menu::{StandardItem, SubMenu};
use ksni::{MenuItem, ToolTip, TrayMethods};
use oclock::dto::state::{ExportedState, TaskInfo};
use std::sync::Arc;
use tokio::sync::mpsc;

/// Messages sent from the tray to the Iced application.
#[derive(Debug, Clone)]
pub enum TrayEvent {
    /// The tray spawned successfully. Contains a handle for state updates.
    Ready(TrayHandle),
    /// The tray failed to spawn.
    SpawnFailed(String),
    /// User clicked the tray icon — toggle the main window.
    ToggleWindow,
    /// User selected a task from the tray context menu.
    SwitchTask(i32),
}

/// Thread-safe handle to update the tray icon state.
#[derive(Clone)]
pub struct TrayHandle {
    handle: Arc<ksni::Handle<OClockTray>>,
}

impl std::fmt::Debug for TrayHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrayHandle").finish_non_exhaustive()
    }
}

impl TrayHandle {
    /// Update the tray state with new daemon state.
    pub async fn update_state(&self, state: &ExportedState) {
        let current_task = state.current_task.clone();
        let all_tasks = state.all_tasks.clone();
        self.handle
            .update(move |tray| {
                tray.current_task = current_task;
                tray.all_tasks = all_tasks;
            })
            .await;
    }
}

/// Internal tray message type (sent from ksni callbacks to the bridge).
#[derive(Debug)]
enum InternalTrayMsg {
    ToggleWindow,
    SwitchTask(i32),
}

/// The ksni tray implementation.
#[derive(Debug)]
struct OClockTray {
    current_task: Option<TaskInfo>,
    all_tasks: Vec<TaskInfo>,
    tx: mpsc::UnboundedSender<InternalTrayMsg>,
}

impl ksni::Tray for OClockTray {
    fn id(&self) -> String {
        "oclock-ui".into()
    }

    fn icon_name(&self) -> String {
        "clock".into()
    }

    fn title(&self) -> String {
        "OClock".into()
    }

    fn tool_tip(&self) -> ToolTip {
        let description = match &self.current_task {
            Some(task) => format!("Current task: <b>{}</b>", task.name),
            None => "No active task".into(),
        };
        ToolTip {
            title: "OClock".into(),
            description,
            ..Default::default()
        }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.tx.send(InternalTrayMsg::ToggleWindow);
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let mut items: Vec<MenuItem<Self>> = Vec::new();

        // Header: current task status
        let status_label = match &self.current_task {
            Some(task) => format!("● {}", task.name),
            None => "○ Idle".into(),
        };
        items.push(
            StandardItem {
                label: status_label,
                enabled: false,
                ..Default::default()
            }
            .into(),
        );

        items.push(MenuItem::Separator);

        // Task list for switching
        let enabled_tasks: Vec<&TaskInfo> = self
            .all_tasks
            .iter()
            .filter(|t| t.enabled != 0)
            .collect();

        if enabled_tasks.is_empty() {
            items.push(
                StandardItem {
                    label: "No tasks".into(),
                    enabled: false,
                    ..Default::default()
                }
                .into(),
            );
        } else {
            let current_id = self.current_task.as_ref().map(|t| t.id);
            let task_items: Vec<MenuItem<Self>> = enabled_tasks
                .into_iter()
                .map(|task| {
                    let is_active = current_id == Some(task.id);
                    let label = if is_active {
                        format!("● {}", task.name)
                    } else {
                        format!("  {}", task.name)
                    };
                    let task_id = task.id;
                    StandardItem {
                        label,
                        activate: Box::new(move |this: &mut Self| {
                            let _ = this.tx.send(InternalTrayMsg::SwitchTask(task_id));
                        }),
                        ..Default::default()
                    }
                    .into()
                })
                .collect();

            items.push(
                SubMenu {
                    label: "Switch task".into(),
                    submenu: task_items,
                    ..Default::default()
                }
                .into(),
            );
        }

        items.push(MenuItem::Separator);

        // Show/hide window
        items.push(
            StandardItem {
                label: "Show/Hide".into(),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.tx.send(InternalTrayMsg::ToggleWindow);
                }),
                ..Default::default()
            }
            .into(),
        );

        // Quit
        items.push(
            StandardItem {
                label: "Quit".into(),
                icon_name: "application-exit".into(),
                activate: Box::new(|_| std::process::exit(0)),
                ..Default::default()
            }
            .into(),
        );

        items
    }
}

/// Create an Iced-compatible stream that spawns the tray and yields events.
///
/// The first event is always `TrayEvent::Ready(handle)` or `TrayEvent::SpawnFailed(err)`.
/// Subsequent events are user interactions with the tray (toggle window, switch task).
pub fn tray_stream() -> impl iced::futures::Stream<Item = TrayEvent> {
    iced::stream::channel(32, async |mut sender| {
        use iced::futures::SinkExt;

        let (tx, mut rx) = mpsc::unbounded_channel();

        let tray = OClockTray {
            current_task: None,
            all_tasks: Vec::new(),
            tx,
        };

        let handle = match tray.spawn().await {
            Ok(h) => h,
            Err(err) => {
                let _ = sender.send(TrayEvent::SpawnFailed(err.to_string())).await;
                return;
            }
        };

        let tray_handle = TrayHandle {
            handle: Arc::new(handle),
        };

        if sender.send(TrayEvent::Ready(tray_handle)).await.is_err() {
            return;
        }

        // Forward tray interactions to the Iced app
        while let Some(msg) = rx.recv().await {
            let event = match msg {
                InternalTrayMsg::ToggleWindow => TrayEvent::ToggleWindow,
                InternalTrayMsg::SwitchTask(id) => TrayEvent::SwitchTask(id),
            };
            if sender.send(event).await.is_err() {
                break;
            }
        }
    })
}

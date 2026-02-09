mod config;
mod tray;
mod views;

use config::Config;
use iced::futures::SinkExt;
use iced::widget::{button, center, column, container, row, text};
use iced::{Alignment, Element, Length, Size, Subscription, Task};
use oclock::dto::state::ExportedState;
use oclock_bridge::subscription::DaemonEvent;
use oclock_idle::IdleEvent;
use std::time::Duration;
use tray::{TrayEvent, TrayHandle};
use views::quick_switch::{QuickSwitchMessage, QuickSwitchView};
use views::timeline::{TimelineMessage, TimelineView};

/// How often to poll the idle detector (seconds).
const IDLE_POLL_INTERVAL_SECS: u64 = 5;

/// Idle threshold set once at boot from config, read by the idle subscription.
static IDLE_THRESHOLD: std::sync::OnceLock<u64> = std::sync::OnceLock::new();

fn main() -> iced::Result {
    env_logger::init();

    iced::application(App::boot, App::update, App::view)
        .title(App::title)
        .subscription(App::subscription)
        .window_size(Size::new(360.0, 480.0))
        .run()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Tasks,
    Timeline,
    Detail,
    Export,
}

impl Tab {
    const ALL: &[Tab] = &[Tab::Tasks, Tab::Timeline, Tab::Detail, Tab::Export];

    fn label(&self) -> &'static str {
        match self {
            Tab::Tasks => "Tasks",
            Tab::Timeline => "Timeline",
            Tab::Detail => "Detail",
            Tab::Export => "Export",
        }
    }
}

/// State for the idle-return dialog.
#[derive(Debug, Clone)]
struct IdleReturnDialog {
    idle_duration: Duration,
}

#[derive(Debug, Clone)]
enum Message {
    StateLoaded(Result<ExportedState, String>),
    DaemonEvent(DaemonEvent),
    CommandResult(Result<ExportedState, String>),
    QuickSwitch(QuickSwitchMessage),
    TabSelected(Tab),
    Tray(TrayEvent),
    TraySynced,
    Idle(IdleEvent),
    IdleReturnDismiss,
    IdleReturnKeepCurrent,
    IdleReturnSwitchTask(i32),
    Timeline(TimelineMessage),
}

struct App {
    config: Config,
    state: Option<ExportedState>,
    error: Option<String>,
    subscribed: bool,
    active_tab: Tab,
    quick_switch: QuickSwitchView,
    timeline: TimelineView,
    tray_handle: Option<TrayHandle>,
    idle_dialog: Option<IdleReturnDialog>,
}

impl App {
    fn boot() -> (Self, Task<Message>) {
        let config = Config::load();
        IDLE_THRESHOLD.set(config.idle.threshold_minutes).ok();

        let app = Self {
            config,
            state: None,
            error: None,
            subscribed: false,
            active_tab: Tab::Tasks,
            quick_switch: QuickSwitchView::new(),
            timeline: TimelineView::new(),
            tray_handle: None,
            idle_dialog: None,
        };

        let init_state = Task::perform(oclock_bridge::commands::get_state(), |result| {
            Message::StateLoaded(result.map_err(|e| e.to_string()))
        });

        (app, init_state)
    }

    fn title(&self) -> String {
        match &self.state {
            Some(state) => match &state.current_task {
                Some(task) => format!("OClock - {}", task.name),
                None => "OClock - idle".into(),
            },
            None => "OClock".into(),
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::StateLoaded(Ok(state)) => {
                self.state = Some(state);
                self.error = None;
                self.subscribed = true;
                self.sync_tray()
            }
            Message::StateLoaded(Err(err)) => {
                log::error!("Failed to load initial state: {err}");
                self.error = Some(err);
                Task::none()
            }
            Message::DaemonEvent(DaemonEvent::StateUpdated(state)) => {
                self.state = Some(state);
                self.error = None;
                let tray = self.sync_tray();
                // Refresh timeline if it's the active tab (state changed = task switched)
                let timeline = if self.active_tab == Tab::Timeline {
                    self.fetch_timeline()
                } else {
                    Task::none()
                };
                Task::batch([tray, timeline])
            }
            Message::DaemonEvent(DaemonEvent::Disconnected) => {
                self.error = Some("Disconnected from daemon".into());
                self.subscribed = false;
                Task::none()
            }
            Message::CommandResult(Ok(state)) => {
                self.state = Some(state);
                self.error = None;
                let tray = self.sync_tray();
                let timeline = if self.active_tab == Tab::Timeline {
                    self.fetch_timeline()
                } else {
                    Task::none()
                };
                Task::batch([tray, timeline])
            }
            Message::CommandResult(Err(err)) => {
                log::error!("Command failed: {err}");
                self.error = Some(err);
                Task::none()
            }
            Message::QuickSwitch(qs_msg) => self.handle_quick_switch(qs_msg),
            Message::TabSelected(tab) => {
                self.active_tab = tab;
                if tab == Tab::Timeline && self.timeline.needs_fetch() {
                    self.fetch_timeline()
                } else {
                    Task::none()
                }
            }
            Message::Tray(event) => self.handle_tray(event),
            Message::TraySynced => Task::none(),
            Message::Idle(event) => self.handle_idle(event),
            Message::IdleReturnDismiss => {
                self.idle_dialog = None;
                Task::none()
            }
            Message::IdleReturnKeepCurrent => {
                self.idle_dialog = None;
                Task::none()
            }
            Message::IdleReturnSwitchTask(id) => {
                self.idle_dialog = None;
                let idle_since = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    .saturating_sub(self.config.idle.threshold_minutes * 60);
                Task::perform(
                    oclock_bridge::commands::retro_switch_task(id as u64, idle_since, true),
                    |r| Message::CommandResult(r.map_err(|e| e.to_string())),
                )
            }
            Message::Timeline(tl_msg) => self.handle_timeline(tl_msg),
        }
    }

    fn handle_timeline(&mut self, msg: TimelineMessage) -> Task<Message> {
        match msg {
            TimelineMessage::BlocksLoaded(_) => {
                self.timeline.update(msg);
                Task::none()
            }
            _ => {
                // Navigation messages — update state then fetch
                self.timeline.update(msg);
                self.fetch_timeline()
            }
        }
    }

    fn fetch_timeline(&mut self) -> Task<Message> {
        self.timeline.set_loading();
        let (start, end) = self.timeline.day_range();
        Task::perform(
            oclock_bridge::commands::events_by_range(start, end),
            |r| Message::Timeline(TimelineMessage::BlocksLoaded(r.map_err(|e| e.to_string()))),
        )
    }

    fn handle_idle(&mut self, event: IdleEvent) -> Task<Message> {
        match event {
            IdleEvent::IdleStarted => {
                log::info!("User went idle");
                Task::none()
            }
            IdleEvent::UserReturned { idle_duration } => {
                log::info!("User returned after {}s idle", idle_duration.as_secs());
                self.idle_dialog = Some(IdleReturnDialog { idle_duration });
                Task::none()
            }
        }
    }

    fn handle_tray(&mut self, event: TrayEvent) -> Task<Message> {
        match event {
            TrayEvent::Ready(handle) => {
                log::info!("System tray icon ready");
                self.tray_handle = Some(handle);
                self.sync_tray()
            }
            TrayEvent::SpawnFailed(err) => {
                log::warn!("Failed to spawn system tray: {err}");
                Task::none()
            }
            TrayEvent::ToggleWindow => {
                log::info!("Tray: toggle window");
                Task::none()
            }
            TrayEvent::SwitchTask(id) => Task::perform(
                oclock_bridge::commands::switch_task(id as u64),
                |r| Message::CommandResult(r.map_err(|e| e.to_string())),
            ),
        }
    }

    fn sync_tray(&self) -> Task<Message> {
        if let (Some(handle), Some(state)) = (&self.tray_handle, &self.state) {
            let handle = handle.clone();
            let state = state.clone();
            Task::perform(
                async move { handle.update_state(&state).await },
                |_| Message::TraySynced,
            )
        } else {
            Task::none()
        }
    }

    fn handle_quick_switch(&mut self, msg: QuickSwitchMessage) -> Task<Message> {
        match msg {
            QuickSwitchMessage::SwitchTask(id) => Task::perform(
                oclock_bridge::commands::switch_task(id as u64),
                |r| Message::CommandResult(r.map_err(|e| e.to_string())),
            ),
            QuickSwitchMessage::DisableTask(id) => Task::perform(
                oclock_bridge::commands::disable_task(id as u64),
                |r| Message::CommandResult(r.map_err(|e| e.to_string())),
            ),
            QuickSwitchMessage::SubmitNewTask => {
                let name = self.quick_switch.take_new_task_name();
                if name.trim().is_empty() {
                    return Task::none();
                }
                self.quick_switch.update(QuickSwitchMessage::SubmitNewTask);
                Task::perform(oclock_bridge::commands::push_task(name), |r| {
                    Message::CommandResult(r.map_err(|e| e.to_string()))
                })
            }
            other => {
                self.quick_switch.update(other);
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        // Show idle-return dialog as an overlay if present
        if let Some(ref dialog) = self.idle_dialog {
            return self.view_idle_dialog(dialog);
        }

        if let Some(ref err) = self.error {
            return center(
                column![
                    text("OClock").size(24),
                    text(format!("Error: {err}")).size(14),
                ]
                .spacing(10),
            )
            .into();
        }

        let Some(ref state) = self.state else {
            return center(
                column![
                    text("OClock").size(24),
                    text("Connecting to daemon...").size(14),
                ]
                .spacing(10),
            )
            .into();
        };

        let tab_bar = self.tab_bar();

        let tab_content: Element<'_, Message> = match self.active_tab {
            Tab::Tasks => self.quick_switch.view(state).map(Message::QuickSwitch),
            Tab::Timeline => self.timeline.view(&self.config).map(Message::Timeline),
            _ => center(text("Coming soon").size(14)).into(),
        };

        column![
            tab_bar,
            container(tab_content)
                .padding([0, 16])
                .height(Length::Fill),
        ]
        .into()
    }

    fn view_idle_dialog(&self, dialog: &IdleReturnDialog) -> Element<'_, Message> {
        let minutes = dialog.idle_duration.as_secs() / 60;
        let header = text(format!("You were idle for ~{minutes} minutes")).size(18);

        let mut col = column![header, text("What would you like to do?").size(14),]
            .spacing(16)
            .padding(24)
            .align_x(Alignment::Center);

        col = col.push(
            button(text("Keep current task").size(14))
                .on_press(Message::IdleReturnKeepCurrent)
                .style(button::primary)
                .width(Length::Fill),
        );

        if let Some(ref state) = self.state {
            for task in state.all_tasks.iter().filter(|t| t.enabled != 0) {
                let is_current = state.current_task.as_ref().is_some_and(|c| c.id == task.id);
                if !is_current {
                    let task_id = task.id;
                    col = col.push(
                        button(text(format!("Switch to: {}", task.name)).size(14))
                            .on_press(Message::IdleReturnSwitchTask(task_id))
                            .style(button::secondary)
                            .width(Length::Fill),
                    );
                }
            }
        }

        col = col.push(
            button(text("Dismiss").size(14))
                .on_press(Message::IdleReturnDismiss)
                .style(button::text)
                .width(Length::Fill),
        );

        center(container(col).width(320).padding(8)).into()
    }

    fn tab_bar(&self) -> Element<'_, Message> {
        let tabs = Tab::ALL.iter().map(|tab| {
            let label = text(tab.label()).size(13);
            let btn = button(label)
                .on_press(Message::TabSelected(*tab))
                .padding([8, 12])
                .style(if *tab == self.active_tab {
                    button::primary
                } else {
                    button::text
                });
            btn.into()
        });

        container(
            row(tabs).spacing(2).align_y(Alignment::Center),
        )
        .padding([8, 12])
        .width(Length::Fill)
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        let mut subs = vec![
            Subscription::run(tray::tray_stream).map(Message::Tray),
            Subscription::run(idle_stream).map(Message::Idle),
        ];

        if self.subscribed {
            subs.push(Subscription::run(daemon_stream).map(Message::DaemonEvent));
        }

        Subscription::batch(subs)
    }
}

fn daemon_stream() -> impl iced::futures::Stream<Item = DaemonEvent> {
    iced::stream::channel(32, async |mut sender| {
        loop {
            match oclock_bridge::subscription::start() {
                Ok(mut rx) => {
                    while let Some(event) = rx.recv().await {
                        if sender.send(event).await.is_err() {
                            return;
                        }
                    }
                    let _ = sender.send(DaemonEvent::Disconnected).await;
                }
                Err(err) => {
                    log::error!("Failed to subscribe to daemon: {err}");
                    let _ = sender.send(DaemonEvent::Disconnected).await;
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    })
}

fn idle_stream() -> impl iced::futures::Stream<Item = IdleEvent> {
    iced::stream::channel(4, async move |mut sender| {
        let threshold_minutes = IDLE_THRESHOLD.get().copied().unwrap_or(5);
        let threshold = Duration::from_secs(threshold_minutes * 60);
        let poll_interval = Duration::from_secs(IDLE_POLL_INTERVAL_SECS);

        let monitor = tokio::task::spawn_blocking(move || {
            oclock_idle::detect().map(|detector| oclock_idle::IdleMonitor::new(detector, threshold))
        })
        .await
        .expect("idle detect task panicked");

        let Some(mut monitor) = monitor else {
            log::info!("No idle detector available, idle subscription disabled");
            std::future::pending::<()>().await;
            return;
        };

        loop {
            let event = tokio::task::spawn_blocking(move || {
                let evt = monitor.poll();
                (monitor, evt)
            })
            .await
            .expect("idle poll task panicked");

            monitor = event.0;
            if let Some(idle_event) = event.1 {
                if sender.send(idle_event).await.is_err() {
                    return;
                }
            }

            tokio::time::sleep(poll_interval).await;
        }
    })
}

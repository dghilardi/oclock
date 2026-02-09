mod views;

use iced::futures::SinkExt;
use iced::widget::{button, center, column, container, row, text};
use iced::{Alignment, Element, Length, Size, Subscription, Task};
use oclock::dto::state::ExportedState;
use oclock_bridge::subscription::DaemonEvent;
use views::quick_switch::{QuickSwitchMessage, QuickSwitchView};

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

#[derive(Debug, Clone)]
enum Message {
    StateLoaded(Result<ExportedState, String>),
    DaemonEvent(DaemonEvent),
    CommandResult(Result<ExportedState, String>),
    QuickSwitch(QuickSwitchMessage),
    TabSelected(Tab),
}

struct App {
    state: Option<ExportedState>,
    error: Option<String>,
    subscribed: bool,
    active_tab: Tab,
    quick_switch: QuickSwitchView,
}

impl App {
    fn boot() -> (Self, Task<Message>) {
        let app = Self {
            state: None,
            error: None,
            subscribed: false,
            active_tab: Tab::Tasks,
            quick_switch: QuickSwitchView::new(),
        };

        let init_task = Task::perform(oclock_bridge::commands::get_state(), |result| {
            Message::StateLoaded(result.map_err(|e| e.to_string()))
        });

        (app, init_task)
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
                Task::none()
            }
            Message::StateLoaded(Err(err)) => {
                log::error!("Failed to load initial state: {err}");
                self.error = Some(err);
                Task::none()
            }
            Message::DaemonEvent(DaemonEvent::StateUpdated(state)) => {
                self.state = Some(state);
                self.error = None;
                Task::none()
            }
            Message::DaemonEvent(DaemonEvent::Disconnected) => {
                self.error = Some("Disconnected from daemon".into());
                self.subscribed = false;
                Task::none()
            }
            Message::CommandResult(Ok(state)) => {
                self.state = Some(state);
                self.error = None;
                Task::none()
            }
            Message::CommandResult(Err(err)) => {
                log::error!("Command failed: {err}");
                self.error = Some(err);
                Task::none()
            }
            Message::QuickSwitch(qs_msg) => self.handle_quick_switch(qs_msg),
            Message::TabSelected(tab) => {
                self.active_tab = tab;
                Task::none()
            }
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
            Tab::Tasks => self
                .quick_switch
                .view(state)
                .map(Message::QuickSwitch),
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
            row(tabs)
                .spacing(2)
                .align_y(Alignment::Center),
        )
        .padding([8, 12])
        .width(Length::Fill)
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        if self.subscribed {
            Subscription::run(daemon_stream).map(Message::DaemonEvent)
        } else {
            Subscription::none()
        }
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

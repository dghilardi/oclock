use iced::futures::SinkExt;
use iced::widget::{center, column, container, text};
use iced::{Element, Subscription, Task};
use oclock::dto::state::ExportedState;
use oclock_bridge::subscription::DaemonEvent;

fn main() -> iced::Result {
    env_logger::init();

    iced::application(App::boot, App::update, App::view)
        .title(App::title)
        .subscription(App::subscription)
        .run()
}

#[derive(Debug, Clone)]
enum Message {
    /// Initial state loaded from daemon.
    StateLoaded(Result<ExportedState, String>),
    /// Real-time state update from PUB socket.
    DaemonEvent(DaemonEvent),
}

struct App {
    state: Option<ExportedState>,
    error: Option<String>,
    subscribed: bool,
}

impl App {
    fn boot() -> (Self, Task<Message>) {
        let app = Self {
            state: None,
            error: None,
            subscribed: false,
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
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let content: Element<Message> = if let Some(ref err) = self.error {
            column![
                text("OClock").size(24),
                text(format!("Error: {err}")).size(14),
            ]
            .spacing(10)
            .into()
        } else if let Some(ref state) = self.state {
            let current = match &state.current_task {
                Some(task) => format!("Current task: {}", task.name),
                None => "No active task".into(),
            };

            let task_count = state.all_tasks.iter().filter(|t| t.enabled == 1).count();

            column![
                text("OClock").size(24),
                text(current).size(18),
                text(format!("{task_count} task(s) registered")).size(14),
            ]
            .spacing(10)
            .into()
        } else {
            column![
                text("OClock").size(24),
                text("Connecting to daemon...").size(14),
            ]
            .spacing(10)
            .into()
        };

        center(container(content).padding(20)).into()
    }

    fn subscription(&self) -> Subscription<Message> {
        if self.subscribed {
            Subscription::run(daemon_stream).map(Message::DaemonEvent)
        } else {
            Subscription::none()
        }
    }
}

/// Creates a stream of DaemonEvents by connecting to the oclock PUB socket.
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
                    // Channel closed — daemon disconnected, try to reconnect.
                    let _ = sender.send(DaemonEvent::Disconnected).await;
                }
                Err(err) => {
                    log::error!("Failed to subscribe to daemon: {err}");
                    let _ = sender.send(DaemonEvent::Disconnected).await;
                }
            }
            // Wait before reconnecting.
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    })
}

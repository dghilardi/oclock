use iced::widget::{button, column, container, row, rule, scrollable, text, text_input, Column};
use iced::{Alignment, Element, Length};
use oclock::dto::state::{ExportedState, TaskInfo};

#[derive(Debug, Clone)]
pub enum QuickSwitchMessage {
    SwitchTask(i32),
    DisableTask(i32),
    NewTaskNameChanged(String),
    SubmitNewTask,
}

pub struct QuickSwitchView {
    new_task_name: String,
}

impl QuickSwitchView {
    pub fn new() -> Self {
        Self {
            new_task_name: String::new(),
        }
    }

    pub fn update(&mut self, message: QuickSwitchMessage) {
        match message {
            QuickSwitchMessage::NewTaskNameChanged(name) => {
                self.new_task_name = name;
            }
            QuickSwitchMessage::SubmitNewTask => {
                // Handled by the parent — we just clear the field.
                self.new_task_name.clear();
            }
            // SwitchTask and DisableTask are handled by the parent.
            _ => {}
        }
    }

    pub fn view<'a>(&'a self, state: &'a ExportedState) -> Element<'a, QuickSwitchMessage> {
        let current_id = state.current_task.as_ref().map(|t| t.id);

        let tasks: Vec<Element<QuickSwitchMessage>> = state
            .all_tasks
            .iter()
            .filter(|t| t.enabled == 1)
            .map(|task| task_row(task, current_id == Some(task.id)))
            .collect();

        let task_list = if tasks.is_empty() {
            column![text("No tasks yet. Create one below.").size(14)]
        } else {
            Column::with_children(tasks).spacing(4)
        };

        let new_task_input = row![
            text_input("New task name...", &self.new_task_name)
                .on_input(QuickSwitchMessage::NewTaskNameChanged)
                .on_submit(QuickSwitchMessage::SubmitNewTask)
                .padding(8),
            button(text("Add").align_x(iced::alignment::Horizontal::Center))
                .on_press_maybe(if self.new_task_name.trim().is_empty() {
                    None
                } else {
                    Some(QuickSwitchMessage::SubmitNewTask)
                })
                .padding([8, 16]),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        column![
            scrollable(container(task_list).padding([0, 4])).height(Length::Fill),
            rule::horizontal(1),
            container(new_task_input).padding([8, 0]),
        ]
        .spacing(8)
        .height(Length::Fill)
        .into()
    }

    pub fn take_new_task_name(&mut self) -> String {
        std::mem::take(&mut self.new_task_name)
    }
}

fn task_row(task: &TaskInfo, is_active: bool) -> Element<'_, QuickSwitchMessage> {
    let indicator = if is_active {
        text("\u{25CF}").size(14) // filled circle
    } else {
        text("\u{25CB}").size(14) // hollow circle
    };

    let name = text(&task.name).size(16).width(Length::Fill);

    let switch_btn = button(
        row![indicator, name]
            .spacing(8)
            .align_y(Alignment::Center),
    )
    .on_press(QuickSwitchMessage::SwitchTask(task.id))
    .padding([10, 12])
    .width(Length::Fill)
    .style(if is_active {
        button::primary
    } else {
        button::secondary
    });

    let disable_btn = button(text("\u{2715}").size(12)) // ✕
        .on_press(QuickSwitchMessage::DisableTask(task.id))
        .padding([10, 10])
        .style(button::danger);

    row![switch_btn, disable_btn]
        .spacing(4)
        .align_y(Alignment::Center)
        .into()
}

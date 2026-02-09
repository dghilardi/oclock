use chrono::{DateTime, Local, NaiveDate, TimeZone};
use iced::widget::{button, column, container, row, scrollable, text, Column};
use iced::{Alignment, Element, Length};
use oclock::dto::state::TimeBlock;

#[derive(Debug, Clone)]
pub enum DetailMessage {
    PreviousDay,
    NextDay,
    Today,
    BlocksLoaded(Result<Vec<TimeBlock>, String>),
    DeleteEvent(i32),
}

pub struct DetailView {
    date: NaiveDate,
    blocks: Vec<TimeBlock>,
    loading: bool,
}

impl DetailView {
    pub fn new() -> Self {
        Self {
            date: Local::now().date_naive(),
            blocks: Vec::new(),
            loading: true,
        }
    }

    pub fn day_range(&self) -> (u64, u64) {
        let start = Local
            .from_local_datetime(&self.date.and_hms_opt(0, 0, 0).unwrap())
            .single()
            .unwrap();
        let end = Local
            .from_local_datetime(&self.date.and_hms_opt(23, 59, 59).unwrap())
            .single()
            .unwrap();
        (start.timestamp() as u64, end.timestamp() as u64 + 1)
    }

    pub fn needs_fetch(&self) -> bool {
        self.loading
    }

    pub fn set_loading(&mut self) {
        self.loading = true;
    }

    pub fn update(&mut self, msg: DetailMessage) {
        match msg {
            DetailMessage::PreviousDay => {
                self.date = self.date.pred_opt().unwrap_or(self.date);
                self.loading = true;
            }
            DetailMessage::NextDay => {
                self.date = self.date.succ_opt().unwrap_or(self.date);
                self.loading = true;
            }
            DetailMessage::Today => {
                self.date = Local::now().date_naive();
                self.loading = true;
            }
            DetailMessage::BlocksLoaded(Ok(blocks)) => {
                self.blocks = blocks;
                self.loading = false;
            }
            DetailMessage::BlocksLoaded(Err(_)) => {
                self.blocks = Vec::new();
                self.loading = false;
            }
            DetailMessage::DeleteEvent(_) => {
                // Handled in main.rs
            }
        }
    }

    pub fn view(&self) -> Element<'_, DetailMessage> {
        let date_nav = row![
            button(text("<").size(16))
                .on_press(DetailMessage::PreviousDay)
                .style(button::text)
                .padding([4, 12]),
            button(text("Today").size(13))
                .on_press(DetailMessage::Today)
                .style(button::text)
                .padding([4, 8]),
            text(self.date.format("%A, %d %B %Y").to_string()).size(14),
            button(text(">").size(16))
                .on_press(DetailMessage::NextDay)
                .style(button::text)
                .padding([4, 12]),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        if self.loading {
            return column![date_nav, text("Loading...").size(14)]
                .spacing(12)
                .into();
        }

        if self.blocks.is_empty() {
            return column![date_nav, text("No events for this day").size(14)]
                .spacing(12)
                .into();
        }

        // Table header
        let header = row![
            container(text("Time").size(11))
                .width(Length::FillPortion(3)),
            container(text("Task").size(11))
                .width(Length::FillPortion(3)),
            container(text("Duration").size(11))
                .width(Length::FillPortion(2)),
            container(text("").size(11)).width(Length::FillPortion(1)),
        ]
        .spacing(4)
        .padding([4, 0]);

        let day_start = Local
            .from_local_datetime(&self.date.and_hms_opt(0, 0, 0).unwrap())
            .single()
            .unwrap();
        let day_end_ts = day_start.timestamp() + 86400;

        let mut rows = Column::new().spacing(2);

        for block in &self.blocks {
            let block_start = block.ts_start.max(day_start.timestamp());
            let block_end = block.ts_end.unwrap_or(day_end_ts).min(day_end_ts);
            let duration_secs = (block_end - block_start).max(0);

            let start_local: DateTime<Local> =
                Local.timestamp_opt(block_start, 0).single().unwrap();
            let end_local: DateTime<Local> =
                Local.timestamp_opt(block_end, 0).single().unwrap();

            let time_str = format!(
                "{} - {}",
                start_local.format("%H:%M:%S"),
                end_local.format("%H:%M:%S")
            );

            let task_name = block.task_name.as_deref().unwrap_or("(no task)");

            let duration_mins = duration_secs / 60;
            let duration_str = if duration_mins >= 60 {
                format!("{}h {}m", duration_mins / 60, duration_mins % 60)
            } else {
                format!("{}m {}s", duration_mins, duration_secs % 60)
            };

            let event_id = block.id;
            let event_row = row![
                container(text(time_str).size(11))
                    .width(Length::FillPortion(3)),
                container(text(task_name).size(11))
                    .width(Length::FillPortion(3)),
                container(text(duration_str).size(11))
                    .width(Length::FillPortion(2)),
                container(
                    button(text("x").size(10))
                        .on_press(DetailMessage::DeleteEvent(event_id))
                        .style(button::danger)
                        .padding([2, 6])
                )
                .width(Length::FillPortion(1)),
            ]
            .spacing(4)
            .padding([4, 0])
            .align_y(Alignment::Center);

            rows = rows.push(event_row);
        }

        column![
            date_nav,
            header,
            scrollable(rows.width(Length::Fill)).height(Length::Fill),
        ]
        .spacing(4)
        .into()
    }
}

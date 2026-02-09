use chrono::{DateTime, Local, NaiveDate, TimeZone};
use iced::widget::{button, column, container, row, scrollable, text, Column};
use iced::{Alignment, Color, Element, Length};
use oclock::dto::state::TimeBlock;

use crate::config::Config;

/// Height in pixels per hour on the timeline.
const PIXELS_PER_HOUR: f32 = 60.0;

/// Minimum block height in pixels (so tiny blocks are still visible).
const MIN_BLOCK_HEIGHT: f32 = 4.0;

#[derive(Debug, Clone)]
pub enum TimelineMessage {
    PreviousDay,
    NextDay,
    Today,
    BlocksLoaded(Result<Vec<TimeBlock>, String>),
}

pub struct TimelineView {
    date: NaiveDate,
    blocks: Vec<TimeBlock>,
    loading: bool,
}

impl TimelineView {
    pub fn new() -> Self {
        Self {
            date: Local::now().date_naive(),
            blocks: Vec::new(),
            loading: true,
        }
    }

    /// Returns the unix timestamp range for the current day (local time).
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

    /// Whether we need to fetch new data (date changed or initial load).
    pub fn needs_fetch(&self) -> bool {
        self.loading
    }

    pub fn set_loading(&mut self) {
        self.loading = true;
    }

    pub fn update(&mut self, msg: TimelineMessage) {
        match msg {
            TimelineMessage::PreviousDay => {
                self.date = self.date.pred_opt().unwrap_or(self.date);
                self.loading = true;
            }
            TimelineMessage::NextDay => {
                self.date = self.date.succ_opt().unwrap_or(self.date);
                self.loading = true;
            }
            TimelineMessage::Today => {
                self.date = Local::now().date_naive();
                self.loading = true;
            }
            TimelineMessage::BlocksLoaded(Ok(blocks)) => {
                self.blocks = blocks;
                self.loading = false;
            }
            TimelineMessage::BlocksLoaded(Err(_)) => {
                self.blocks = Vec::new();
                self.loading = false;
            }
        }
    }

    pub fn view<'a>(&'a self, config: &'a Config) -> Element<'a, TimelineMessage> {
        let date_nav = row![
            button(text("<").size(16))
                .on_press(TimelineMessage::PreviousDay)
                .style(button::text)
                .padding([4, 12]),
            button(text("Today").size(13))
                .on_press(TimelineMessage::Today)
                .style(button::text)
                .padding([4, 8]),
            text(self.date.format("%A, %d %B %Y").to_string()).size(14),
            button(text(">").size(16))
                .on_press(TimelineMessage::NextDay)
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
            return column![date_nav, text("No activity for this day").size(14)]
                .spacing(12)
                .into();
        }

        // Build the timeline: hour labels + colored blocks
        let day_start = Local
            .from_local_datetime(&self.date.and_hms_opt(0, 0, 0).unwrap())
            .single()
            .unwrap();
        let day_end_ts = day_start.timestamp() + 86400;

        let mut timeline_rows = Column::new().spacing(0);

        for block in &self.blocks {
            let block_start = block.ts_start.max(day_start.timestamp());
            let block_end = block.ts_end.unwrap_or(day_end_ts).min(day_end_ts);
            let duration_secs = (block_end - block_start).max(0) as f32;
            let duration_hours = duration_secs / 3600.0;
            let height = (duration_hours * PIXELS_PER_HOUR).max(MIN_BLOCK_HEIGHT);

            let start_local: DateTime<Local> =
                Local.timestamp_opt(block_start, 0).single().unwrap();
            let end_local: DateTime<Local> = Local.timestamp_opt(block_end, 0).single().unwrap();

            let time_label = format!(
                "{} - {}",
                start_local.format("%H:%M"),
                end_local.format("%H:%M")
            );

            let task_name = block
                .task_name
                .as_deref()
                .unwrap_or("(no task)");

            let color = block
                .task_id
                .map(|id| parse_hex_color(config.task_color(id)))
                .unwrap_or(Color::from_rgb(0.6, 0.6, 0.6));

            let duration_mins = (duration_secs / 60.0).round() as u32;
            let duration_label = if duration_mins >= 60 {
                format!("{}h {}m", duration_mins / 60, duration_mins % 60)
            } else {
                format!("{}m", duration_mins)
            };

            let block_content = row![
                container(text("").size(1))
                    .width(4)
                    .height(Length::Fill)
                    .style(move |_theme: &_| container::Style {
                        background: Some(color.into()),
                        ..Default::default()
                    }),
                column![
                    text(task_name).size(13),
                    text(format!("{} ({})", time_label, duration_label)).size(11),
                ]
                .spacing(2)
                .padding([4, 8]),
            ]
            .align_y(Alignment::Center);

            timeline_rows = timeline_rows.push(
                container(block_content)
                    .width(Length::Fill)
                    .height(height)
                    .padding([2, 0]),
            );
        }

        // Summary
        let total_secs: i64 = self
            .blocks
            .iter()
            .map(|b| {
                let s = b.ts_start.max(day_start.timestamp());
                let e = b.ts_end.unwrap_or(day_end_ts).min(day_end_ts);
                (e - s).max(0)
            })
            .sum();
        let total_hours = total_secs / 3600;
        let total_mins = (total_secs % 3600) / 60;
        let summary = text(format!("Total: {}h {}m", total_hours, total_mins)).size(13);

        column![
            date_nav,
            summary,
            scrollable(timeline_rows.width(Length::Fill)).height(Length::Fill),
        ]
        .spacing(8)
        .into()
    }
}

fn parse_hex_color(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return Color::from_rgb(0.5, 0.5, 0.5);
    }
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(128) as f32 / 255.0;
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(128) as f32 / 255.0;
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(128) as f32 / 255.0;
    Color::from_rgb(r, g, b)
}

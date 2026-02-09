use chrono::{Local, NaiveDate, TimeZone};
use iced::widget::{button, checkbox, column, container, pick_list, row, scrollable, text};
use iced::{Alignment, Element, Length};
use oclock::dto::state::TimeBlock;

#[derive(Debug, Clone)]
pub enum ExportMessage {
    StartPreviousDay,
    StartNextDay,
    EndPreviousDay,
    EndNextDay,
    RoundingChanged(RoundingOption),
    GroupByTaskToggled(bool),
    Generate,
    BlocksLoaded(Result<Vec<TimeBlock>, String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundingOption {
    None,
    Round5,
    Round15,
    Round30,
}

impl RoundingOption {
    const ALL: &[RoundingOption] = &[
        RoundingOption::None,
        RoundingOption::Round5,
        RoundingOption::Round15,
        RoundingOption::Round30,
    ];

    fn minutes(&self) -> Option<u64> {
        match self {
            RoundingOption::None => None,
            RoundingOption::Round5 => Some(5),
            RoundingOption::Round15 => Some(15),
            RoundingOption::Round30 => Some(30),
        }
    }
}

impl std::fmt::Display for RoundingOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RoundingOption::None => write!(f, "No rounding"),
            RoundingOption::Round5 => write!(f, "5 minutes"),
            RoundingOption::Round15 => write!(f, "15 minutes"),
            RoundingOption::Round30 => write!(f, "30 minutes"),
        }
    }
}

pub struct ExportView {
    start_date: NaiveDate,
    end_date: NaiveDate,
    rounding: RoundingOption,
    group_by_task: bool,
    blocks: Vec<TimeBlock>,
    csv_output: Option<String>,
    loading: bool,
}

impl ExportView {
    pub fn new() -> Self {
        let today = Local::now().date_naive();
        Self {
            start_date: today,
            end_date: today,
            rounding: RoundingOption::None,
            group_by_task: true,
            blocks: Vec::new(),
            csv_output: None,
            loading: false,
        }
    }

    pub fn date_range(&self) -> (u64, u64) {
        let start = Local
            .from_local_datetime(&self.start_date.and_hms_opt(0, 0, 0).unwrap())
            .single()
            .unwrap();
        let end = Local
            .from_local_datetime(&self.end_date.and_hms_opt(23, 59, 59).unwrap())
            .single()
            .unwrap();
        (start.timestamp() as u64, end.timestamp() as u64 + 1)
    }

    pub fn update(&mut self, msg: ExportMessage) {
        match msg {
            ExportMessage::StartPreviousDay => {
                self.start_date = self.start_date.pred_opt().unwrap_or(self.start_date);
                self.csv_output = None;
            }
            ExportMessage::StartNextDay => {
                let next = self.start_date.succ_opt().unwrap_or(self.start_date);
                if next <= self.end_date {
                    self.start_date = next;
                }
                self.csv_output = None;
            }
            ExportMessage::EndPreviousDay => {
                let prev = self.end_date.pred_opt().unwrap_or(self.end_date);
                if prev >= self.start_date {
                    self.end_date = prev;
                }
                self.csv_output = None;
            }
            ExportMessage::EndNextDay => {
                self.end_date = self.end_date.succ_opt().unwrap_or(self.end_date);
                self.csv_output = None;
            }
            ExportMessage::RoundingChanged(r) => {
                self.rounding = r;
                self.csv_output = None;
            }
            ExportMessage::GroupByTaskToggled(v) => {
                self.group_by_task = v;
                self.csv_output = None;
            }
            ExportMessage::Generate => {
                self.loading = true;
                self.csv_output = None;
            }
            ExportMessage::BlocksLoaded(Ok(blocks)) => {
                self.blocks = blocks;
                self.loading = false;
                self.csv_output = Some(self.generate_csv());
            }
            ExportMessage::BlocksLoaded(Err(_)) => {
                self.blocks = Vec::new();
                self.loading = false;
                self.csv_output = Some("Error loading data".into());
            }
        }
    }

    fn generate_csv(&self) -> String {
        if self.group_by_task {
            self.generate_grouped_csv()
        } else {
            self.generate_flat_csv()
        }
    }

    fn generate_flat_csv(&self) -> String {
        let mut lines = vec!["date,task,start,end,duration_hours".to_string()];

        for block in &self.blocks {
            let task = block.task_name.as_deref().unwrap_or("(no task)");
            let start_dt = chrono::DateTime::from_timestamp(block.ts_start, 0);
            let end_ts = block.ts_end.unwrap_or(block.ts_start);
            let end_dt = chrono::DateTime::from_timestamp(end_ts, 0);
            let duration_secs = (end_ts - block.ts_start).max(0) as f64;
            let duration_hours = self.round_duration(duration_secs / 3600.0);

            let date_str = start_dt
                .map(|d| d.with_timezone(&Local).format("%Y-%m-%d").to_string())
                .unwrap_or_default();
            let start_str = start_dt
                .map(|d| d.with_timezone(&Local).format("%H:%M").to_string())
                .unwrap_or_default();
            let end_str = end_dt
                .map(|d| d.with_timezone(&Local).format("%H:%M").to_string())
                .unwrap_or_default();

            lines.push(format!(
                "{},{},{},{},{:.2}",
                date_str, task, start_str, end_str, duration_hours
            ));
        }

        lines.join("\n")
    }

    fn generate_grouped_csv(&self) -> String {
        use std::collections::BTreeMap;

        // Group by (date, task) and sum durations
        let mut groups: BTreeMap<(String, String), f64> = BTreeMap::new();

        for block in &self.blocks {
            let task = block
                .task_name
                .as_deref()
                .unwrap_or("(no task)")
                .to_string();
            let date_str = chrono::DateTime::from_timestamp(block.ts_start, 0)
                .map(|d| d.with_timezone(&Local).format("%Y-%m-%d").to_string())
                .unwrap_or_default();
            let end_ts = block.ts_end.unwrap_or(block.ts_start);
            let duration_secs = (end_ts - block.ts_start).max(0) as f64;

            *groups.entry((date_str, task)).or_default() += duration_secs;
        }

        let mut lines = vec!["date,task,duration_hours".to_string()];
        for ((date, task), secs) in &groups {
            let hours = self.round_duration(*secs / 3600.0);
            lines.push(format!("{},{},{:.2}", date, task, hours));
        }

        lines.join("\n")
    }

    fn round_duration(&self, hours: f64) -> f64 {
        match self.rounding.minutes() {
            None => hours,
            Some(mins) => {
                let fraction = mins as f64 / 60.0;
                (hours / fraction).round() * fraction
            }
        }
    }

    pub fn view(&self) -> Element<'_, ExportMessage> {
        let start_row = row![
            text("From:").size(13),
            button(text("<").size(14))
                .on_press(ExportMessage::StartPreviousDay)
                .style(button::text)
                .padding([2, 8]),
            text(self.start_date.format("%Y-%m-%d").to_string()).size(13),
            button(text(">").size(14))
                .on_press(ExportMessage::StartNextDay)
                .style(button::text)
                .padding([2, 8]),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        let end_row = row![
            text("To:    ").size(13),
            button(text("<").size(14))
                .on_press(ExportMessage::EndPreviousDay)
                .style(button::text)
                .padding([2, 8]),
            text(self.end_date.format("%Y-%m-%d").to_string()).size(13),
            button(text(">").size(14))
                .on_press(ExportMessage::EndNextDay)
                .style(button::text)
                .padding([2, 8]),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        let rounding_row = row![
            text("Rounding:").size(13),
            pick_list(
                RoundingOption::ALL,
                Some(self.rounding),
                ExportMessage::RoundingChanged
            )
            .text_size(13),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let group_row = checkbox(self.group_by_task)
            .label("Group by task per day")
            .on_toggle(ExportMessage::GroupByTaskToggled)
            .text_size(13);

        let generate_btn = button(text("Generate CSV").size(14))
            .on_press(ExportMessage::Generate)
            .style(button::primary)
            .width(Length::Fill);

        let mut col = column![start_row, end_row, rounding_row, group_row, generate_btn,]
            .spacing(10);

        if self.loading {
            col = col.push(text("Generating...").size(13));
        }

        if let Some(ref csv) = self.csv_output {
            col = col.push(
                container(
                    scrollable(text(csv).size(11)).height(Length::Fill),
                )
                .padding(8)
                .width(Length::Fill),
            );
        }

        col.into()
    }
}

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Application configuration, stored at `$XDG_CONFIG_HOME/oclock-ui/config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub idle: IdleConfig,
    pub window: WindowConfig,
    pub tasks: TasksConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct IdleConfig {
    /// Idle threshold in minutes before showing the return dialog.
    pub threshold_minutes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowConfig {
    /// Start the app minimized to the system tray.
    pub start_minimized: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TasksConfig {
    /// Explicit color overrides per task ID (hex string, e.g. "#4A90D9").
    #[serde(default)]
    pub colors: HashMap<i32, String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            idle: IdleConfig::default(),
            window: WindowConfig::default(),
            tasks: TasksConfig::default(),
        }
    }
}

impl Default for IdleConfig {
    fn default() -> Self {
        Self {
            threshold_minutes: 5,
        }
    }
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            start_minimized: false,
        }
    }
}

impl Default for TasksConfig {
    fn default() -> Self {
        Self {
            colors: HashMap::new(),
        }
    }
}

/// Auto-assigned color palette for tasks without explicit overrides.
const PALETTE: &[&str] = &[
    "#4A90D9", "#E67E22", "#2ECC71", "#E74C3C", "#9B59B6", "#1ABC9C", "#F39C12", "#3498DB",
    "#D35400", "#27AE60", "#C0392B", "#8E44AD", "#16A085", "#F1C40F", "#2980B9",
];

impl Config {
    /// Load config from the XDG config path, or return defaults if not found.
    pub fn load() -> Self {
        let Some(path) = config_path() else {
            log::warn!("Could not determine XDG config directory, using defaults");
            return Self::default();
        };

        match std::fs::read_to_string(&path) {
            Ok(contents) => match toml::from_str(&contents) {
                Ok(config) => {
                    log::info!("Loaded config from {}", path.display());
                    config
                }
                Err(err) => {
                    log::warn!("Failed to parse config at {}: {err}", path.display());
                    Self::default()
                }
            },
            Err(_) => {
                log::info!("No config file at {}, using defaults", path.display());
                Self::default()
            }
        }
    }

    /// Get the color for a task, using explicit override or auto-assigning from palette.
    pub fn task_color(&self, task_id: i32) -> &str {
        if let Some(color) = self.tasks.colors.get(&task_id) {
            color.as_str()
        } else {
            PALETTE[task_id.unsigned_abs() as usize % PALETTE.len()]
        }
    }
}

fn config_path() -> Option<PathBuf> {
    ProjectDirs::from("", "", "oclock-ui").map(|dirs| dirs.config_dir().join("config.toml"))
}

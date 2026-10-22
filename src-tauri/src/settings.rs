//! User settings, stored as a small JSON file in the app's config directory.

use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Origin of the Odoo server, e.g. `https://odoo.example.org`. `None` = show the connect screen.
    pub server_url: Option<String>,
    /// Path (and fragment) of Discuss on that server; depends on the Odoo version.
    pub discuss_path: String,
    pub notifications: bool,
    pub show_preview: bool,
    pub play_sound: bool,
    pub show_badge: bool,
    /// Closing the window hides it (the app keeps receiving messages from the menu bar / tray).
    pub keep_running: bool,
    pub start_at_login: bool,
    pub quiet_hours: bool,
    /// "HH:MM", local time.
    pub quiet_from: String,
    pub quiet_to: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            server_url: None,
            discuss_path: "/odoo/discuss".into(),
            notifications: true,
            show_preview: true,
            play_sound: true,
            show_badge: true,
            keep_running: true,
            start_at_login: false,
            quiet_hours: false,
            quiet_from: "22:00".into(),
            quiet_to: "07:00".into(),
        }
    }
}

fn path(app: &AppHandle) -> Option<PathBuf> {
    let dir = app.path().app_config_dir().ok()?;
    fs::create_dir_all(&dir).ok()?;
    Some(dir.join("settings.json"))
}

pub fn load(app: &AppHandle) -> Settings {
    path(app)
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    let p = path(app).ok_or("Cannot find the configuration folder")?;
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(p, json).map_err(|e| e.to_string())
}

/// True when `now` ("HH:MM") falls inside the quiet period, which may cross midnight.
pub fn in_quiet_hours(settings: &Settings, now: &str) -> bool {
    if !settings.quiet_hours {
        return false;
    }
    let (from, to) = (settings.quiet_from.as_str(), settings.quiet_to.as_str());
    if from == to {
        return false;
    }
    if from < to {
        now >= from && now < to
    } else {
        now >= from || now < to
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quiet(from: &str, to: &str) -> Settings {
        Settings { quiet_hours: true, quiet_from: from.into(), quiet_to: to.into(), ..Settings::default() }
    }

    #[test]
    fn quiet_hours_across_midnight() {
        let s = quiet("22:00", "07:00");
        assert!(in_quiet_hours(&s, "23:30"));
        assert!(in_quiet_hours(&s, "02:00"));
        assert!(!in_quiet_hours(&s, "07:00"));
        assert!(!in_quiet_hours(&s, "12:00"));
    }

    #[test]
    fn quiet_hours_same_day_and_disabled() {
        let s = quiet("13:00", "14:00");
        assert!(in_quiet_hours(&s, "13:30"));
        assert!(!in_quiet_hours(&s, "14:00"));
        assert!(!in_quiet_hours(&Settings::default(), "03:00"));
        assert!(!in_quiet_hours(&quiet("08:00", "08:00"), "08:00"));
    }
}

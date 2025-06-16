use chrono::Utc;
use colored::*;
use std::io::Write;

use super::constants::{
    PID_DISPLAY_WIDTH, ROOT_UID, UID_DISPLAY_WIDTH, UNKNOWN_UID_DISPLAY, USER_UID,
};

pub struct Logger;

impl Logger {
    pub fn init(debug_level: log::Level) {
        let level_filter = match debug_level {
            log::Level::Error => log::LevelFilter::Error,
            log::Level::Warn => log::LevelFilter::Warn,
            log::Level::Info => log::LevelFilter::Info,
            log::Level::Debug => log::LevelFilter::Debug,
            log::Level::Trace => log::LevelFilter::Trace,
        };
        log::set_max_level(level_filter);
    }

    fn timestamp() -> colored::ColoredString {
        Utc::now().format("%Y-%m-%d %H:%M:%S").to_string().green()
    }

    pub fn info<T: Into<String>>(message: T) {
        println!("{} [INFO] - {}", Self::timestamp(), message.into());
        let _ = std::io::stdout().flush();
    }

    pub fn error<T: Into<String>>(message: T) {
        eprintln!("{} [ERROR] - {}", Self::timestamp(), message.into().red());
        let _ = std::io::stderr().flush();
    }

    pub fn event(uid: Option<u32>, pid: u32, cmd: &str) {
        let uid_display = uid.map_or(UNKNOWN_UID_DISPLAY.to_string(), |u| {
            format!("{:<width$}", u, width = UID_DISPLAY_WIDTH)
        });
        let message = format!(
            "CMD:  UID={} PID={:<width$} | {}",
            uid_display,
            pid,
            cmd,
            width = PID_DISPLAY_WIDTH
        );

        let colored_message = match uid {
            Some(ROOT_UID) => message.red(),
            Some(USER_UID) => message.blue(),
            None => message.yellow(),
            _ => message.normal(),
        };

        println!("{} {}", Self::timestamp(), colored_message);
    }

    pub fn fs<T: Into<String>>(message: T) {
        let colored_message = message.into().white();
        println!("{} [FS] - {}", Self::timestamp(), colored_message);
    }

    pub fn debug<T: Into<String>>(message: T) {
        if log::max_level() >= log::LevelFilter::Debug {
            println!("{} [DEBUG] - {}", Self::timestamp(), message.into().cyan());
        }
    }

    pub fn dbus_event(_name: &str, pid: u32, cmd: &str) {
        // we dont have uid info for dbus events
        let uid_display = UNKNOWN_UID_DISPLAY;
        let message = format!(
            "DBUS: UID={:<width$} PID={:<pid_width$} | {}",
            uid_display,
            pid,
            cmd,
            width = UID_DISPLAY_WIDTH,
            pid_width = PID_DISPLAY_WIDTH
        );

        let colored_message = message.yellow();

        println!("{} {}", Self::timestamp(), colored_message);
        if let Err(e) = std::io::stdout().flush() {
            eprintln!("warning: failed to flush stdout: {}", e);
        }
    }
}

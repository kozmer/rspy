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

    fn timestamp() -> ColoredString {
        unsafe {
            let mut t = 0;
            libc::time(&mut t);
            let tm = libc::localtime(&t);
            format!(
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                (*tm).tm_year + 1900,
                (*tm).tm_mon + 1,
                (*tm).tm_mday,
                (*tm).tm_hour,
                (*tm).tm_min,
                (*tm).tm_sec
            )
            .green()
        }
    }

    pub fn info<T: Into<String>>(message: T) {
        println!("{} [INFO] - {}", Self::timestamp(), message.into());
        let _ = std::io::stdout().flush();
    }

    pub fn error<T: Into<String>>(message: T) {
        eprintln!("{} [ERROR] - {}", Self::timestamp(), message.into().red());
        let _ = std::io::stderr().flush();
    }

    fn format_uid(uid: Option<u32>) -> String {
        uid.map_or(UNKNOWN_UID_DISPLAY.to_string(), |u| {
            format!("{:<width$}", u, width = UID_DISPLAY_WIDTH)
        })
    }

    fn colorize_by_uid(message: String, uid: Option<u32>) -> ColoredString {
        match uid {
            Some(ROOT_UID) => message.red(),
            Some(USER_UID) => message.blue(),
            None => message.yellow(),
            _ => message.normal(),
        }
    }

    fn print_process_event(prefix: &str, uid: Option<u32>, pid: u32, cmd: &str) {
        let message = format!(
            "{}: UID={} PID={:<width$} | {}",
            prefix,
            Self::format_uid(uid),
            pid,
            cmd,
            width = PID_DISPLAY_WIDTH
        );
        println!("{} {}", Self::timestamp(), Self::colorize_by_uid(message, uid));
        let _ = std::io::stdout().flush();
    }

    pub fn event(uid: Option<u32>, pid: u32, cmd: &str) {
        Self::print_process_event("CMD ", uid, pid, cmd);
    }

    pub fn fs<T: Into<String>>(message: T) {
        println!("{} [FS] - {}", Self::timestamp(), message.into().white());
    }

    pub fn debug<T: Into<String>>(message: T) {
        if log::max_level() >= log::LevelFilter::Debug {
            println!("{} [DEBUG] - {}", Self::timestamp(), message.into().cyan());
        }
    }

    pub fn dbus_event(pid: u32, cmd: &str) {
        Self::dbus_event_with_uid(pid, cmd, None);
    }

    pub fn dbus_event_with_uid(pid: u32, cmd: &str, uid: Option<u32>) {
        Self::print_process_event("DBUS", uid, pid, cmd);
    }
}

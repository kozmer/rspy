use dbus::blocking::Connection;
use procfs::process::Process;
use rustc_hash::FxHashSet;
use std::time::Duration;

use crate::core::{
    constants::{DBUS_DEFAULT_SLEEP_MS, DBUS_PROXY_TIMEOUT_SECS},
    error::Result,
    logger::Logger,
};

pub struct DBusScanner {
    printed_processes: FxHashSet<u32>,
    interval: Option<Duration>,
}

fn lookup_uid(pid: u32) -> Option<u32> {
    Process::new(pid as i32)
        .ok()?
        .status()
        .ok()
        .map(|s| s.ruid)
}

impl DBusScanner {
    pub fn new(interval: Option<Duration>) -> Self {
        DBusScanner {
            printed_processes: FxHashSet::default(),
            interval,
        }
    }

    pub fn is_available() -> bool {
        match Connection::new_system() {
            Ok(_) => true,
            Err(e) => {
                Logger::debug(format!("failed to connect to system bus: {}", e));
                match Connection::new_session() {
                    Ok(_) => true,
                    Err(e) => {
                        Logger::debug(format!("failed to connect to session bus: {}", e));
                        false
                    }
                }
            }
        }
    }

    pub fn start_listening(&mut self) -> Result<()> {
        Logger::debug("attempting to connect to system dbus...".to_string());
        let conn = Connection::new_system().map_err(|e| {
            Logger::error(format!("failed to connect to system dbus: {}", e));
            e
        })?;

        let sleep_duration = self
            .interval
            .unwrap_or(Duration::from_millis(DBUS_DEFAULT_SLEEP_MS));
        let proxy_timeout = Duration::from_secs(DBUS_PROXY_TIMEOUT_SECS);

        Logger::debug("creating dbus proxy...".to_string());
        // thanks jkr
        let proxy = conn.with_proxy(
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1/unit/_2d_2eslice",
            proxy_timeout,
        );

        Logger::debug("starting dbus monitoring loop...".to_string());
        loop {
            Logger::debug("polling dbus for processes...".to_string());
            match proxy.method_call("org.freedesktop.systemd1.Slice", "GetProcesses", ()) {
                Ok(result) => {
                    let (processes,): (Vec<(String, u32, String)>,) = result;
                    Logger::debug(format!("retrieved {} processes from dbus", processes.len()));

                    for (_name, pid, cmdline) in processes {
                        if self.printed_processes.insert(pid) {
                            let uid = lookup_uid(pid);
                            Logger::dbus_event_with_uid(pid, &cmdline, uid);
                        }
                    }
                }
                Err(e) => {
                    Logger::error(format!("failed to get processes from dbus: {}", e));
                    return Err(e.into());
                }
            }

            std::thread::sleep(sleep_duration);
        }
    }
}

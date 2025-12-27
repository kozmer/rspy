use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::{Duration, Instant};

use crate::core::{
    constants::{DEFAULT_SCAN_INTERVAL_MS, SCANNER_MAX_TIMEOUT_SECS},
    logger::Logger,
};
use crate::monitoring::{dbus::DBusScanner, process::ProcessScanner};

pub struct Scanner {
    interval: Option<Duration>,
    dbus_interval: Option<Duration>,
    trigger_rx: Option<Receiver<()>>,
    is_active: Arc<AtomicBool>,
    dbus_only: bool,
    dbus_scanner: Option<DBusScanner>,
    process_scanner: ProcessScanner,
}

impl Scanner {
    pub fn new(
        interval: Option<Duration>,
        trigger_rx: Receiver<()>,
        dbus_only: bool,
        dbus_enabled: bool,
        dbus_interval: Option<Duration>,
    ) -> Self {
        let dbus_scanner = if dbus_only || dbus_enabled {
            Some(DBusScanner::new(dbus_interval))
        } else {
            None
        };

        Self {
            interval,
            dbus_interval,
            trigger_rx: Some(trigger_rx),
            is_active: Arc::new(AtomicBool::new(false)),
            dbus_only,
            dbus_scanner,
            process_scanner: ProcessScanner::new(),
        }
    }

    pub fn start(&mut self) {
        self.set_active(true);

        if let Some(mut dbus_scanner) = self.dbus_scanner.take() {
            thread::spawn(move || {
                if let Err(e) = dbus_scanner.start_listening() {
                    Logger::error(format!("dbus scanner error: {}", e));
                }
            });
        }

        if self.dbus_only {
            return;
        }

        let is_active = Arc::clone(&self.is_active);
        let interval = self.interval;
        let dbus_interval = self.dbus_interval;
        let mut process_scanner = std::mem::take(&mut self.process_scanner);

        if let Some(trigger_rx) = self.trigger_rx.take() {
            thread::spawn(move || {
                let mut last_process_scan = Instant::now();
                let min_between_scans =
                    interval.unwrap_or(Duration::from_millis(DEFAULT_SCAN_INTERVAL_MS));

                // for inactive sleep, use the lowest of the scanning intervals for responsiveness
                let inactive_sleep_duration = match (interval, dbus_interval) {
                    (Some(proc_int), Some(dbus_int)) => std::cmp::min(proc_int, dbus_int),
                    (Some(proc_int), None) => proc_int,
                    (None, Some(dbus_int)) => dbus_int,
                    (None, None) => Duration::from_millis(DEFAULT_SCAN_INTERVAL_MS),
                };

                loop {
                    if !is_active.load(Ordering::Relaxed) {
                        thread::sleep(inactive_sleep_duration);
                        continue;
                    }

                    let now = Instant::now();
                    let time_since_last_process = now.duration_since(last_process_scan);

                    // calc next process scan time if applicable
                    let next_process_scan =
                        interval.map(|interval_duration| last_process_scan + interval_duration);

                    let timeout = if let Some(next_scan_time) = next_process_scan {
                        if now >= next_scan_time {
                            Duration::from_millis(0)
                        } else {
                            std::cmp::min(
                                next_scan_time.duration_since(now),
                                Duration::from_secs(SCANNER_MAX_TIMEOUT_SECS),
                            )
                        }
                    } else {
                        Duration::from_secs(SCANNER_MAX_TIMEOUT_SECS)
                    };

                    if let Some(next_scan_time) = next_process_scan
                        && now >= next_scan_time
                    {
                        Logger::debug("starting interval-based process scan...".to_string());
                        match process_scanner.scan_processes() {
                            Ok(new_count) => {
                                Logger::debug(format!(
                                    "interval scan completed. Found {} new processes. Time since last scan: {:?}",
                                    new_count, time_since_last_process
                                ));
                            }
                            Err(e) => {
                                Logger::error(format!("interval scan failed: {}", e));
                            }
                        }
                        last_process_scan = Instant::now();
                        continue;
                    }

                    match trigger_rx.recv_timeout(timeout) {
                        Ok(()) => {
                            if time_since_last_process >= min_between_scans {
                                // drain any additional pending triggers to avoid backlog
                                let mut trigger_count = 1;
                                while trigger_rx.try_recv().is_ok() {
                                    trigger_count += 1;
                                }

                                if trigger_count > 1 {
                                    Logger::debug(format!(
                                        "drained {} pending triggers, starting triggered process scan...",
                                        trigger_count
                                    ));
                                } else {
                                    Logger::debug(
                                        "trigger received, starting triggered process scan..."
                                            .to_string(),
                                    );
                                }

                                match process_scanner.scan_processes() {
                                    Ok(new_count) => {
                                        Logger::debug(format!(
                                            "triggered scan completed. Found {} new processes",
                                            new_count
                                        ));
                                    }
                                    Err(e) => {
                                        Logger::error(format!("triggered scan failed: {}", e));
                                    }
                                }
                                last_process_scan = Instant::now();
                            } else {
                                Logger::debug(format!(
                                    "ignoring trigger - only {:?} since last scan (min: {:?})",
                                    time_since_last_process, min_between_scans
                                ));
                            }
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                            continue;
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                            Logger::error("trigger channel disconnected");
                            break;
                        }
                    }
                }
            });
        }
    }

    pub fn set_active(&self, active: bool) {
        self.is_active.store(active, Ordering::Relaxed);
    }
}

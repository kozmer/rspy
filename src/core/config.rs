use clap::Parser;
use std::time::Duration;

use super::constants::{DEFAULT_RECURSIVE_DIRS, DEFAULT_SCAN_INTERVAL_MS, LOW_RESOURCE_WATCH_DIRS};

#[derive(Parser)]
#[command(name = "rspy")]
pub struct Config {
    #[arg(short = 'f', long = "print-filesystem-events")]
    #[arg(help = "enables printing file system events to stdout (disabled by default)")]
    pub print_filesystem_events: bool,

    #[arg(short = 'r', long = "recursive-watch")]
    #[arg(help = "list of directories to watch with Inotify recursively")]
    pub recursive_watch_dirs: Vec<String>,

    #[arg(short = 'd', long = "direct-watch")]
    #[arg(help = "list of directories to watch with inotify directly, not the subdirectories")]
    pub direct_watch_dirs: Vec<String>,

    #[arg(long)]
    #[arg(
        help = "low-resource mode: only monitors /etc and /etc/ld.so.cache with no scan interval"
    )]
    pub low_resource: bool,

    #[arg(long = "scan-interval")]
    #[arg(help = "interval in milliseconds between procfs scans")]
    pub scan_interval_ms: Option<u64>,

    #[arg(long = "dbus-interval")]
    #[arg(help = "interval in milliseconds between DBUS polls")]
    pub dbus_interval_ms: Option<u64>,

    #[arg(long)]
    #[arg(help = "enables debug level logging")]
    pub debug: bool,

    #[arg(long)]
    #[arg(help = "enable dbus monitoring")]
    pub dbus: bool,

    #[arg(long = "dbus-only")]
    #[arg(help = "use only dbus monitoring (disables proc scanning + inotify)")]
    pub dbus_only: bool,

    #[arg(long = "no-interval")]
    #[arg(help = "disable periodic scanning, only trigger scans on filesystem events")]
    pub no_interval: bool,
}

impl Config {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let config = Self::parse();
        config.validate().unwrap_or_else(|e| {
            eprintln!("configuration error: {}", e);
            std::process::exit(1);
        });
        config
    }

    pub fn scan_interval(&self) -> Option<Duration> {
        if self.no_interval {
            None
        } else {
            let interval_ms = self.scan_interval_ms.unwrap_or(DEFAULT_SCAN_INTERVAL_MS);
            Some(Duration::from_millis(interval_ms))
        }
    }

    pub fn dbus_interval(&self) -> Option<Duration> {
        self.dbus_interval_ms
            .map(Duration::from_millis)
            .or_else(|| {
                Some(Duration::from_millis(
                    super::constants::DBUS_DEFAULT_SLEEP_MS,
                ))
            })
    }

    pub fn get_direct_watch_dirs(&self) -> Vec<String> {
        let mut dirs = self.direct_watch_dirs.clone();
        if self.low_resource {
            dirs.extend(LOW_RESOURCE_WATCH_DIRS.iter().map(|&s| s.to_string()));
        }
        dirs
    }

    pub fn get_recursive_watch_dirs(&self) -> Vec<String> {
        if !self.recursive_watch_dirs.is_empty() {
            return self.recursive_watch_dirs.clone();
        }

        if !self.low_resource && self.direct_watch_dirs.is_empty() {
            DEFAULT_RECURSIVE_DIRS
                .iter()
                .map(|&s| s.to_string())
                .collect()
        } else {
            Vec::new()
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.low_resource {
            if !self.recursive_watch_dirs.is_empty() {
                return Err(
                    "--low-resource cannot be used with --recursive-watch directories".to_string(),
                );
            }
            if !self.direct_watch_dirs.is_empty() {
                return Err(
                    "--low-resource cannot be used with --direct-watch directories".to_string(),
                );
            }
        }

        Ok(())
    }
}


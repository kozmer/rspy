pub mod core;
pub mod monitoring;
pub mod utils;

use crate::core::config::Config;
use crate::core::error::Result;
use crate::core::logger::Logger;
use crate::monitoring::{dbus::DBusScanner, filesystem::FsWatcher, scanner::Scanner};
use crate::utils::format::format_duration;

use colored::*;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, channel};

struct Runtime {
    config: Config,
    running: Arc<AtomicBool>,
}

impl Runtime {
    fn new(config: Config) -> Self {
        Self {
            config,
            running: Arc::new(AtomicBool::new(true)),
        }
    }

    fn display_banner_and_config(&self) -> Result<()> {
        let version = env!("CARGO_PKG_VERSION");
        let git_commit_sha = option_env!("GIT_COMMIT_HASH").unwrap_or("unknown");

        println!(
            "rspy - version: {} - commit sha: {}",
            version, git_commit_sha
        );

        println!(
            "{}",
            "
 ██▀███    ██████  ██▓███ ▓██   ██▓
▓██ ▒ ██▒▒██    ▒ ▓██░  ██▒▒██  ██▒
▓██ ░▄█ ▒░ ▓██▄   ▓██░ ██▓▒ ▒██ ██░
▒██▀▀█▄    ▒   ██▒▒██▄█▓▒ ▒ ░ ▐██▓░
░██▓ ▒██▒▒██████▒▒▒██▒ ░  ░ ░ ██▒▓░
░ ▒▓ ░▒▓░▒ ▒▓▒ ▒ ░▒▓▒░ ░  ░  ██▒▒▒ 
  ░▒ ░ ▒░░ ░▒  ░ ░░▒ ░     ▓██ ░▒░ 
  ░░   ░ ░  ░  ░  ░░       ▒ ▒ ░░  
   ░           ░           ░ ░     
                           ░ ░
        "
            .red()
        );

        self.display_config_info()
    }

    fn display_config_info(&self) -> Result<()> {
        println!("\n{}", "configuration:".cyan().bold());
        println!(
            "  print file system events: {}",
            if self.config.print_filesystem_events {
                "enabled".green()
            } else {
                "disabled".red()
            }
        );

        if self.config.dbus_only {
            println!("  process scanning: {}", "dbus only".yellow());
        } else {
            match self.config.scan_interval() {
                Some(interval) => println!(
                    "  process scanning: {}",
                    format!("every {} + inotify events", format_duration(Some(interval))).green()
                ),
                None => println!("  process scanning: {}", "inotify events only".green()),
            }
        }

        if !self.config.dbus_only {
            println!("  watch directories:");
            if !self.config.get_recursive_watch_dirs().is_empty() {
                println!(
                    "    recursive: {:?}",
                    self.config.get_recursive_watch_dirs()
                );
            }
            if !self.config.get_direct_watch_dirs().is_empty() {
                println!("    direct: {:?}", self.config.get_direct_watch_dirs());
            }
        }

        println!(
            "  dbus monitoring: {}",
            if self.config.dbus || self.config.dbus_only {
                "enabled".green()
            } else {
                "disabled".red()
            }
        );

        if self.config.dbus || self.config.dbus_only {
            println!(
                "  dbus scan interval: {}",
                format_duration(self.config.dbus_interval()).cyan()
            );
        }

        if !self.config.dbus_only {
            println!(
                "  low-resource mode: {}",
                if self.config.low_resource {
                    "enabled".green()
                } else {
                    "disabled".red()
                }
            );
        }

        Ok(())
    }

    fn confirm_configuration(&self) -> Result<bool> {
        loop {
            print!("\nproceed with this configuration? [y/n]: ");
            if let Err(e) = io::stdout().flush() {
                eprintln!("Warning: Failed to flush stdout: {}", e);
            }

            let mut input = String::new();
            if io::stdin().read_line(&mut input).is_err() {
                println!("failed to read input. exiting...");
                return Ok(false);
            }
            let input = input.trim().to_lowercase();

            match input.as_str() {
                "y" | "yes" => return Ok(true),
                "n" | "no" => {
                    println!("exiting...");
                    return Ok(false);
                }
                _ => {
                    println!("invalid input. please enter 'y' or 'n'");
                    continue;
                }
            }
        }
    }

    fn setup_signal_handler(&self) -> Result<()> {
        let running = self.running.clone();
        ctrlc::set_handler(move || {
            Logger::info("received interrupt signal, shutting down...".to_string());
            running.store(false, Ordering::SeqCst);
        })
        .map_err(|e| format!("error setting Ctrl-C handler: {}", e))?;
        Ok(())
    }

    fn run(self) -> Result<()> {
        self.display_banner_and_config()?;

        if !self.confirm_configuration()? {
            std::process::exit(0);
        }

        println!();
        self.setup_signal_handler()?;

        if (self.config.dbus || self.config.dbus_only) && !DBusScanner::is_available() {
            Logger::error("dbus is not available on this system. exiting...".to_string());
            std::process::exit(1);
        }

        let (tx, rx) = channel();
        let (trigger_tx, trigger_rx) = mpsc::channel();

        let directories: Vec<PathBuf> = self
            .config
            .get_recursive_watch_dirs()
            .iter()
            .map(PathBuf::from)
            .collect();

        let mut fs_watcher = if !self.config.dbus_only {
            Some(FsWatcher::new(
                tx.clone(),
                trigger_tx,
                directories,
                self.config
                    .get_direct_watch_dirs()
                    .iter()
                    .map(PathBuf::from)
                    .collect(),
                self.config.print_filesystem_events,
                self.config.low_resource,
                self.config.debug,
            )?)
        } else {
            None
        };

        if let Some(watcher) = fs_watcher.as_mut()
            && let Err(e) = watcher.setup_watches()
        {
            Logger::error(format!("failed to setup filesystem watches: {}", e));
            std::process::exit(1);
        }

        let mut scanner = Scanner::new(
            self.config.scan_interval(),
            trigger_rx,
            self.config.dbus_only,
            self.config.dbus,
            self.config.dbus_interval(),
        );

        scanner.set_active(true);
        scanner.start();

        if let Some(watcher) = fs_watcher
            && let Err(e) = watcher.start_watching()
        {
            Logger::error(format!("failed to start filesystem watcher: {}", e));
            std::process::exit(1);
        }

        self.event_loop(rx)
    }

    fn event_loop(self, rx: Receiver<String>) -> Result<()> {
        loop {
            if !self.running.load(Ordering::SeqCst) {
                Logger::info("shutting down gracefully...".to_string());
                break;
            }

            match rx.recv_timeout(std::time::Duration::from_millis(100)) {
                Ok(event) => {
                    if self.config.print_filesystem_events {
                        Logger::fs(event);
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    continue;
                }
                Err(e) => {
                    Logger::error(format!("event channel disconnected: {}", e));
                    break;
                }
            }
        }

        Logger::info("rspy terminated".to_string());
        Ok(())
    }
}

fn main() {
    let config = Config::new();
    Logger::init(if config.debug {
        log::Level::Debug
    } else {
        log::Level::Info
    });

    let runtime = Runtime::new(config);

    if let Err(e) = runtime.run() {
        Logger::error(format!("runtime error: {}", e));
        std::process::exit(1);
    }
}

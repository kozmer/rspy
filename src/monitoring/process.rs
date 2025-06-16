use procfs::process::{Process, all_processes};
use std::collections::HashSet;

use crate::core::{
    constants::{DEFAULT_NEW_PIDS_CAPACITY, UNKNOWN_COMMAND},
    error::Result,
    logger::Logger,
};

pub struct ProcessScanner {
    seen_pids: HashSet<i32>,
    #[allow(dead_code)]
    current_pids: HashSet<i32>,
    #[allow(dead_code)]
    new_pids: Vec<i32>,
}

impl ProcessScanner {
    pub fn new() -> Self {
        Self {
            seen_pids: HashSet::new(),
            current_pids: HashSet::new(),
            new_pids: Vec::new(),
        }
    }

    pub fn scan_processes(&mut self) -> Result<usize> {
        let processes = all_processes()?;

        self.current_pids.clear();
        self.current_pids.reserve(processes.len());
        self.new_pids.clear();
        self.new_pids.reserve(DEFAULT_NEW_PIDS_CAPACITY);

        for process in processes {
            let pid = process.pid();
            self.current_pids.insert(pid);

            if self.seen_pids.insert(pid) {
                self.new_pids.push(pid);
            }
        }

        let mut new_count = 0;
        for &pid in &self.new_pids {
            match self.process_new_pid(pid) {
                Ok(()) => new_count += 1,
                Err(e) => {
                    Logger::debug(format!("failed to process pid {}: {}", pid, e));
                    self.seen_pids.remove(&pid);
                    continue;
                }
            }
        }

        self.seen_pids.retain(|pid| self.current_pids.contains(pid));

        Ok(new_count)
    }

    fn process_new_pid(&self, pid: i32) -> Result<()> {
        let process = Process::new(pid)?;

        let cmdline = process
            .cmdline()
            .unwrap_or_else(|_| vec![UNKNOWN_COMMAND.to_string()])
            .join(" ");

        let status = process.status()?;
        let uid = status.ruid;

        Logger::event(Some(uid), pid as u32, &cmdline);
        Ok(())
    }

    pub fn get_process_count(&self) -> usize {
        self.seen_pids.len()
    }
}

impl Default for ProcessScanner {
    fn default() -> Self {
        Self::new()
    }
}

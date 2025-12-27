use libc::{self, IN_ALL_EVENTS, IN_OPEN, inotify_add_watch, inotify_init1};
use rustc_hash::FxHashMap;
use std::io;
use std::os::unix::io::RawFd;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::thread;
use walkdir::WalkDir;

use crate::core::{error::Result, logger::Logger};

const BUFFER_SIZE: usize = 1024;

const IN_ACCESS: u32 = 0x00000001;
const IN_MODIFY: u32 = 0x00000002;
const IN_ATTRIB: u32 = 0x00000004;
const IN_CLOSE_WRITE: u32 = 0x00000008;
const IN_CLOSE_NOWRITE: u32 = 0x00000010;
const IN_MOVED_FROM: u32 = 0x00000040;
const IN_MOVED_TO: u32 = 0x00000080;
const IN_CREATE: u32 = 0x00000100;
const IN_DELETE: u32 = 0x00000200;

#[repr(C)]
struct InotifyEvent {
    wd: i32,
    mask: u32,
    cookie: u32,
    len: u32,
    name: [u8; 0],
}

pub struct FsWatcher {
    fd: RawFd,
    sender: Sender<String>,
    trigger_sender: Sender<()>,
    recursive_directories: Vec<PathBuf>,
    direct_directories: Vec<PathBuf>,
    print_events: bool,
    low_resource: bool,
    debug: bool,
    wd_to_path: FxHashMap<i32, PathBuf>,
}

impl FsWatcher {
    fn get_event_string(mask: u32) -> String {
        let mut events = Vec::new();

        if mask & IN_ACCESS != 0 {
            events.push("ACCESS");
        }
        if mask & IN_MODIFY != 0 {
            events.push("MODIFY");
        }
        if mask & IN_ATTRIB != 0 {
            events.push("ATTRIB");
        }
        if mask & IN_CLOSE_WRITE != 0 {
            events.push("CLOSE_WRITE");
        }
        if mask & IN_CLOSE_NOWRITE != 0 {
            events.push("CLOSE_NOWRITE");
        }
        if mask & IN_OPEN != 0 {
            events.push("OPEN");
        }
        if mask & IN_MOVED_FROM != 0 {
            events.push("MOVED_FROM");
        }
        if mask & IN_MOVED_TO != 0 {
            events.push("MOVED_TO");
        }
        if mask & IN_CREATE != 0 {
            events.push("CREATE");
        }
        if mask & IN_DELETE != 0 {
            events.push("DELETE");
        }

        events.join("|")
    }

    pub fn new(
        sender: Sender<String>,
        trigger_sender: Sender<()>,
        recursive_directories: Vec<PathBuf>,
        direct_directories: Vec<PathBuf>,
        print_events: bool,
        low_resource: bool,
        debug: bool,
    ) -> Result<Self> {
        let fd = unsafe { inotify_init1(0) };
        if fd == -1 {
            return Err(io::Error::last_os_error().into());
        }

        Ok(Self {
            fd,
            sender,
            trigger_sender,
            recursive_directories,
            direct_directories,
            print_events,
            low_resource,
            debug,
            wd_to_path: FxHashMap::default(),
        })
    }

    pub fn setup_watches(&mut self) -> Result<()> {
        let recursive_dirs = self.recursive_directories.clone();
        let direct_dirs = self.direct_directories.clone();

        for directory in recursive_dirs {
            self.add_watch(&directory, true)?;
        }

        for directory in direct_dirs {
            self.add_watch(&directory, false)?;
        }

        Ok(())
    }

    fn add_watch(&mut self, path: &Path, is_recursive: bool) -> Result<()> {
        if is_recursive {
            for entry in WalkDir::new(path)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_dir())
            {
                self.add_watch_single(entry.path())?;
            }
        } else {
            self.add_watch_single(path)?;
        }
        Ok(())
    }

    fn add_watch_single(&mut self, path: &Path) -> Result<()> {
        let path_str = match path.to_str() {
            Some(s) => std::ffi::CString::new(s)
                .map_err(|e| format!("failed to create CString for path {:?}: {}", path, e))?,
            None => {
                Logger::error(format!("path contains invalid UTF-8: {:?}", path));
                return Ok(());
            }
        };

        let wd = unsafe {
            inotify_add_watch(
                self.fd,
                path_str.as_ptr(),
                if self.low_resource {
                    IN_OPEN
                } else {
                    IN_ALL_EVENTS
                },
            )
        };

        if wd != -1 {
            self.wd_to_path.insert(wd, path.to_path_buf());
            if self.debug {
                Logger::debug(format!("watching: {:?} (wd={})", path, wd));
            }
        } else {
            let err = io::Error::last_os_error();
            if self.debug || err.kind() != io::ErrorKind::PermissionDenied {
                Logger::error(format!("failed to monitor {:?}: {}", path, err));
            }
        }
        Ok(())
    }

    pub fn start_watching(self) -> Result<()> {
        let sender = self.sender.clone();
        let trigger_sender = self.trigger_sender.clone();
        let wd_to_path = self.wd_to_path.clone();
        let print_events = self.print_events;
        let fd = self.fd;
        let debug = self.debug;

        thread::spawn(move || {
            let _watcher = self;
            let mut buffer = [0u8; BUFFER_SIZE];

            loop {
                let read_result = read_events(fd, &mut buffer);

                match read_result {
                    Ok(read_size) => {
                        let mut offset = 0;
                        let mut has_events = false;

                        while offset < read_size {
                            let event =
                                unsafe { &*(buffer.as_ptr().add(offset) as *const InotifyEvent) };

                            has_events = true;

                            if print_events
                                && let Some(path) = wd_to_path.get(&event.wd)
                            {
                                let event_str = format!(
                                    "events: {} on {:?}",
                                    Self::get_event_string(event.mask),
                                    path
                                );
                                if let Err(e) = sender.send(event_str) {
                                    Logger::error(format!("failed to send event: {}", e));
                                }
                            }

                            if debug && let Some(path) = wd_to_path.get(&event.wd) {
                                Logger::debug(format!(
                                    "inotify event: mask={:x} ({}) on {:?}",
                                    event.mask,
                                    Self::get_event_string(event.mask),
                                    path
                                ));
                            }

                            offset += std::mem::size_of::<InotifyEvent>() + event.len as usize;
                        }

                        // send only one trigger per batch of events to avoid flooding
                        if has_events {
                            if let Err(e) = trigger_sender.send(()) {
                                Logger::error(format!("failed to send trigger: {}", e));
                            } else if debug {
                                Logger::debug(
                                    "sent process scan trigger due to filesystem events"
                                        .to_string(),
                                );
                            }
                        }
                    }
                    Err(e) => {
                        Logger::error(format!("error reading events: {}", e));
                        break;
                    }
                }
            }
        });

        Ok(())
    }
}

fn read_events(fd: RawFd, buffer: &mut [u8]) -> io::Result<usize> {
    let read_size =
        unsafe { libc::read(fd, buffer.as_mut_ptr() as *mut libc::c_void, buffer.len()) };

    if read_size < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(read_size as usize)
    }
}

impl Drop for FsWatcher {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.fd);
        }
    }
}

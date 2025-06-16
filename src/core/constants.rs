pub const DEFAULT_SCAN_INTERVAL_MS: u64 = 100;

pub const FS_WATCHER_POLL_INTERVAL_MS: u64 = 100;

pub const SCANNER_MAX_TIMEOUT_SECS: u64 = 1;

pub const DEFAULT_NEW_PIDS_CAPACITY: usize = 32;

pub const DEFAULT_RECURSIVE_DIRS: &[&str] = &["/usr", "/tmp", "/etc", "/home", "/var", "/opt"];

pub const LOW_RESOURCE_WATCH_DIRS: &[&str] = &["/etc/ld.so.cache"];

pub const DBUS_PROXY_TIMEOUT_SECS: u64 = 5;
pub const DBUS_DEFAULT_SLEEP_MS: u64 = 100;

pub const UNKNOWN_UID_DISPLAY: &str = "???";
pub const UNKNOWN_COMMAND: &str = "<unknown command>";
pub const UID_DISPLAY_WIDTH: usize = 5;
pub const PID_DISPLAY_WIDTH: usize = 8;

pub const ROOT_UID: u32 = 0;
pub const USER_UID: u32 = 1000;

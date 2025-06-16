use std::time::Duration;

pub fn format_duration(duration: Option<Duration>) -> String {
    match duration {
        Some(duration) => {
            let total_seconds = duration.as_secs();
            let hours = total_seconds / 3600;
            let minutes = (total_seconds % 3600) / 60;
            let seconds = total_seconds % 60;
            let milliseconds = duration.subsec_millis();

            match (hours, minutes, seconds, milliseconds) {
                (0, 0, 0, ms) => format!("{}ms", ms),
                (0, 0, s, ms) => format!("{}.{:03}s", s, ms),
                (0, m, s, ms) => format!("{}m{:02}.{:03}s", m, s, ms),
                (h, m, s, ms) => format!("{}h{:02}m{:02}.{:03}s", h, m, s, ms),
            }
        }
        None => "disabled".to_string(),
    }
}

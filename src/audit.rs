//! Append-only audit log. Never writes secret bytes or combo sequences.
//!
//! Log location: `$XDG_DATA_HOME/comboauth/audit.log`
//! (defaults to `~/.local/share/comboauth/audit.log`).

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub enum AuditEvent<'a> {
    Activated { service_name: &'a str, delivery_mode: &'a str },
    Failed { reason: FailReason },
}

pub enum FailReason {
    NoMatch,
    TimingMismatch,
    SecretUnavailable,
}

impl FailReason {
    fn as_str(&self) -> &'static str {
        match self {
            FailReason::NoMatch => "NoMatch",
            FailReason::TimingMismatch => "TimingMismatch",
            FailReason::SecretUnavailable => "SecretUnavailable",
        }
    }
}

/// Write an audit event to the log file. Silently drops on any I/O error.
pub fn log(event: AuditEvent<'_>) {
    let Some(path) = log_path() else { return };
    let Some(dir) = path.parent() else { return };
    if fs::create_dir_all(dir).is_err() { return; }

    let ts = iso8601_now();
    let line = match event {
        AuditEvent::Activated { service_name, delivery_mode } =>
            format!("{ts} ACTIVATED service={service_name} delivery={delivery_mode}\n"),
        AuditEvent::Failed { reason } =>
            format!("{ts} FAILED reason={}\n", reason.as_str()),
    };

    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = f.write_all(line.as_bytes());
    }
}

fn log_path() -> Option<PathBuf> {
    let base = std::env::var("XDG_DATA_HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| dirs_next().map(|h| h.join(".local/share")))?;
    Some(base.join("comboauth").join("audit.log"))
}

fn dirs_next() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}

fn iso8601_now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Simple UTC ISO-8601 without chrono dependency
    let (y, mo, d, h, mi, s) = epoch_to_ymdhms(secs);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

fn epoch_to_ymdhms(secs: u64) -> (u32, u32, u32, u32, u32, u32) {
    let s = secs % 60;
    let mins = secs / 60;
    let mi = mins % 60;
    let hours = mins / 60;
    let h = hours % 24;
    let days = (hours / 24) as u32;

    // Days since 1970-01-01
    let mut year = 1970u32;
    let mut remaining = days;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if remaining < days_in_year { break; }
        remaining -= days_in_year;
        year += 1;
    }
    let months = [31u32, if is_leap(year) { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1u32;
    for &dm in &months {
        if remaining < dm { break; }
        remaining -= dm;
        month += 1;
    }
    (year, month, remaining + 1, h as u32, mi as u32, s as u32)
}

fn is_leap(y: u32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso8601_epoch_zero() {
        assert_eq!(iso8601_now_from(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn iso8601_known_date() {
        // 2024-01-15 12:30:45 UTC = 1705321845
        assert_eq!(iso8601_now_from(1705321845), "2024-01-15T12:30:45Z");
    }

    fn iso8601_now_from(secs: u64) -> String {
        let (y, mo, d, h, mi, s) = epoch_to_ymdhms(secs);
        format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
    }

    #[test]
    fn fail_reason_strs() {
        assert_eq!(FailReason::NoMatch.as_str(), "NoMatch");
        assert_eq!(FailReason::TimingMismatch.as_str(), "TimingMismatch");
        assert_eq!(FailReason::SecretUnavailable.as_str(), "SecretUnavailable");
    }
}

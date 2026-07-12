//! On-disk logging for tester bug reports: timestamped app log with simple
//! size rotation, plus a crash-log writer for the panic hook.

use std::io::Write;
use std::path::{Path, PathBuf};

const ROTATE_BYTES: u64 = 2 * 1024 * 1024;

pub struct FileLog {
    path: PathBuf,
}

impl FileLog {
    /// Open (creating dirs) and rotate the previous log if it grew too big.
    pub fn new(logs_dir: &Path) -> Option<Self> {
        std::fs::create_dir_all(logs_dir).ok()?;
        let path = logs_dir.join("optiscaler-gui.log");
        if path
            .metadata()
            .map(|m| m.len() > ROTATE_BYTES)
            .unwrap_or(false)
        {
            let _ = std::fs::rename(&path, logs_dir.join("optiscaler-gui.old.log"));
        }
        Some(Self { path })
    }

    pub fn append(&self, line: &str) {
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            let _ = writeln!(file, "[{}] {line}", timestamp());
        }
    }

    pub fn dir(&self) -> Option<&Path> {
        self.path.parent()
    }
}

fn timestamp() -> String {
    let now = time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    let format = time::macros::format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
    now.format(&format).unwrap_or_default()
}

/// Write a crash report (panic payload + location + backtrace) into the logs
/// dir. Used by the GUI's panic hook so testers always have something to attach.
pub fn write_crash_log(logs_dir: &Path, info: &std::panic::PanicHookInfo<'_>) {
    let _ = std::fs::create_dir_all(logs_dir);
    let path = logs_dir.join("crash.log");
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let backtrace = std::backtrace::Backtrace::force_capture();
        let _ = writeln!(
            file,
            "==== crash at {} (version {}) ====\n{info}\n{backtrace}\n",
            timestamp(),
            crate::VERSION,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_timestamped_lines_and_rotates() {
        let tmp = tempfile::tempdir().unwrap();
        let log = FileLog::new(tmp.path()).unwrap();
        log.append("hello");
        log.append("world");
        let content = std::fs::read_to_string(tmp.path().join("optiscaler-gui.log")).unwrap();
        assert!(content.contains("] hello"));
        assert!(content.contains("] world"));
        assert_eq!(content.lines().count(), 2);
    }
}

//! The operational log.
//!
//! Tarcie is write-only: nothing on screen reports that delivery has stopped,
//! and nothing reads the queue back. Whatever the application has to say goes
//! here, where an operator can find it afterwards.
//!
//! **No capture content is ever written here.** The log records what happened
//! to events, never what was in them. A line that would carry content is a
//! defect, not a feature: the log lives outside the queue, so content in it is
//! a copy of the user's notes that nothing in the design accounts for.

use crate::constraints::{MAX_LOG_BYTES, MAX_LOG_LINE_CHARS};
use anyhow::{Context, Result};
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// Appended to a line that was shortened, so the cut is visible.
const TRUNCATION_MARKER: &str = " […truncated]";

/// A bounded append-only log file.
pub struct LogFile {
    path: PathBuf,
    previous: PathBuf,
    max_bytes: u64,
    lock: Mutex<()>,
}

impl LogFile {
    /// Open the log in `dir`, bounded at the standard ceiling.
    pub fn new_in(dir: &Path) -> Result<Self> {
        Self::with_ceiling(dir, MAX_LOG_BYTES)
    }

    /// Open the log in `dir` with an explicit ceiling.
    ///
    /// `new_in` supplies the standard one. Taking it directly lets a test
    /// prove the rotation without writing a megabyte to do it.
    pub fn with_ceiling(dir: &Path, max_bytes: u64) -> Result<Self> {
        fs::create_dir_all(dir).context("create the log dir")?;

        Ok(Self {
            path: dir.join("tarcie.log"),
            previous: dir.join("tarcie.1.log"),
            max_bytes,
            lock: Mutex::new(()),
        })
    }

    /// Append a line.
    ///
    /// A log that cannot be written is never worth failing a capture over, so
    /// every failure here stops at stderr.
    pub fn write(&self, args: fmt::Arguments<'_>) {
        if let Err(e) = self.try_write(args) {
            eprintln!("tarcie: could not write the log: {e}");
        }
    }

    fn try_write(&self, args: fmt::Arguments<'_>) -> Result<()> {
        // A poisoned lock means an earlier write panicked. The log carries on
        // rather than spreading that panic to whatever is reporting now.
        let _g = self.lock.lock().unwrap_or_else(|e| e.into_inner());

        self.rotate_if_full()?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .context("open the log")?;

        writeln!(file, "{} {}", stamp(), clamp_line(args)).context("write a log line")?;

        // No fsync. The log is a diagnostic, not the durability contract, and
        // an fsync on every line would put the disk in the capture path.
        Ok(())
    }

    fn rotate_if_full(&self) -> Result<()> {
        let full = match fs::metadata(&self.path) {
            Ok(meta) => meta.len() >= self.max_bytes,
            // No log yet, so nothing to rotate.
            Err(_) => false,
        };

        if full {
            // Replacing the previous log is deliberate, and is why this does
            // not use the queue's `free_path`. Two bounded files are worth
            // more than every line ever written plus a full disk.
            fs::rename(&self.path, &self.previous).context("rotate the log")?;
        }

        Ok(())
    }
}

fn stamp() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// Shorten a line that runs past the ceiling.
///
/// A deferral carries the sink's response text verbatim, and how long that
/// runs is the sink's decision, not tarcie's.
fn clamp_line(args: fmt::Arguments<'_>) -> String {
    let text = fmt::format(args);

    if text.chars().count() <= MAX_LOG_LINE_CHARS {
        return text;
    }

    let mut out: String = text.chars().take(MAX_LOG_LINE_CHARS).collect();
    out.push_str(TRUNCATION_MARKER);
    out
}

static LOG: OnceLock<LogFile> = OnceLock::new();

/// Point the log at a directory. Called once, at startup.
pub fn init_in(dir: &Path) -> Result<()> {
    let file = LogFile::new_in(dir)?;

    LOG.set(file)
        .map_err(|_| anyhow::anyhow!("the log is already open"))?;

    Ok(())
}

/// Report something.
///
/// Always to stderr, so a run from a terminal still shows it. Also to the log
/// file once `init_in` has run. Before that — and throughout the tests, which
/// never call it — there is no file, and nothing touches the user's profile.
pub fn write(args: fmt::Arguments<'_>) {
    eprintln!("tarcie: {args}");

    if let Some(log) = LOG.get() {
        log.write(args);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn temp_log(max_bytes: u64) -> (LogFile, TempDir) {
        let dir = TempDir::new().expect("create temp dir");
        let log = LogFile::with_ceiling(&dir.path().join("logs"), max_bytes).expect("open the log");
        (log, dir)
    }

    fn lines(log: &LogFile) -> Vec<String> {
        fs::read_to_string(&log.path)
            .expect("read the log")
            .lines()
            .map(str::to_string)
            .collect()
    }

    #[test]
    fn a_report_reaches_the_file() {
        let (log, _dir) = temp_log(MAX_LOG_BYTES);

        log.write(format_args!("flush deferred"));

        let lines = lines(&log);
        assert_eq!(lines.len(), 1);
        assert!(
            lines[0].ends_with("flush deferred"),
            "the report is kept whole: {}",
            lines[0]
        );
    }

    #[test]
    fn every_line_carries_the_time_it_happened() {
        // A log of delivery failures is worth little without saying when.
        let (log, _dir) = temp_log(MAX_LOG_BYTES);

        log.write(format_args!("something happened"));

        let line = lines(&log).remove(0);
        let (stamped, _) = line.split_once(' ').expect("a stamp and a message");
        assert!(
            stamped.starts_with("20") && stamped.ends_with('Z'),
            "the line opens with a UTC stamp: {stamped}"
        );
    }

    #[test]
    fn reports_accumulate_rather_than_replace() {
        let (log, _dir) = temp_log(MAX_LOG_BYTES);

        log.write(format_args!("first"));
        log.write(format_args!("second"));

        let lines = lines(&log);
        assert_eq!(lines.len(), 2);
        assert!(lines[0].ends_with("first"));
        assert!(lines[1].ends_with("second"));
    }

    #[test]
    fn a_full_log_rotates_and_keeps_one_previous_file() {
        let (log, _dir) = temp_log(120);

        // Enough lines to pass the ceiling more than once.
        for i in 0..40 {
            log.write(format_args!("line {i}"));
        }

        assert!(log.path.exists(), "the live log is there");
        assert!(log.previous.exists(), "one previous log is kept");
        assert!(
            fs::read_to_string(&log.path).expect("read live").len() < 400,
            "the live log stays near its ceiling"
        );
    }

    #[test]
    fn an_over_long_report_is_shortened() {
        // The sink decides how long its response text runs. One line never
        // gets to be the reason the log fills.
        let (log, _dir) = temp_log(MAX_LOG_BYTES);

        log.write(format_args!("sink error: {}", "x".repeat(MAX_LOG_LINE_CHARS * 2)));

        let line = lines(&log).remove(0);
        assert!(line.ends_with(TRUNCATION_MARKER), "the cut is disclosed");
        assert!(
            line.chars().count() < MAX_LOG_LINE_CHARS * 2,
            "the line actually shrank"
        );
    }

    #[test]
    fn a_log_that_cannot_be_written_does_not_take_the_caller_down() {
        // Something else holds the name. Reporting must still be a call that
        // returns, because a capture is on the other end of it.
        let dir = TempDir::new().expect("create temp dir");
        let logs = dir.path().join("logs");
        let log = LogFile::with_ceiling(&logs, MAX_LOG_BYTES).expect("open the log");
        fs::create_dir(&log.path).expect("occupy the log name with a directory");

        log.write(format_args!("this cannot be written"));
    }
}

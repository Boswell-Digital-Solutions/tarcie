//! When a delivery on a daily schedule is owed.
//!
//! The background flusher normally runs on an interval, which suits a sink
//! that is always there. A nightly batch is a different question: not "how
//! long since the last one" but "has today's happened yet".
//!
//! The difference matters on a desktop, which is off for a good part of the
//! day. An interval only ever fires while the application is running, so a
//! machine asleep at the target time would simply skip it. The rule here is
//! held against the calendar instead, so a missed night is delivered at the
//! next opportunity rather than lost until the night after.
//!
//! Everything here is pure or file-local, so the rule is provable without a
//! running desktop session.

use anyhow::{Context, Result};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use std::fs;
use std::path::Path;

/// Read a target time of day, written `HH:MM` on a 24-hour clock.
///
/// The time is local. A nightly batch is a statement about the user's night,
/// not about UTC.
pub fn parse_time_of_day(raw: &str) -> Option<NaiveTime> {
    let raw = raw.trim();
    let (hours, minutes) = raw.split_once(':')?;

    let hours: u32 = hours.parse().ok()?;
    let minutes: u32 = minutes.parse().ok()?;

    NaiveTime::from_hms_opt(hours, minutes, 0)
}

/// Whether today's delivery is still owed.
///
/// Two rules, and the second is what makes a missed night recoverable:
///
/// - The target time has passed today.
/// - Today is not the day already recorded.
///
/// A run that has never delivered on a schedule is owed one immediately. An
/// installation upgrading into a schedule may be holding a backlog, and making
/// it wait a whole day for the first delivery would be a poor trade.
///
/// The recorded day is compared for difference rather than for being earlier.
/// A clock that moves backwards would otherwise stop delivery until the date
/// caught up again, and the contract prefers a duplicate to a delay of that
/// kind.
pub fn is_due(now: NaiveDateTime, target: NaiveTime, last: Option<NaiveDate>) -> bool {
    match last {
        None => true,
        Some(day) => now.date() != day && now.time() >= target,
    }
}

/// The day the last scheduled delivery completed, if one has.
///
/// A marker that cannot be read, or holds something that is not a date, counts
/// as no marker at all. That errs towards delivering again, which costs a
/// duplicate the sink deduplicates on `id`, rather than towards a night that
/// silently never arrives.
pub fn last_delivery(path: &Path) -> Option<NaiveDate> {
    let raw = fs::read_to_string(path).ok()?;
    NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d").ok()
}

/// Record the day a scheduled delivery completed.
pub fn record_delivery(path: &Path, day: NaiveDate) -> Result<()> {
    if let Some(parent) = path.parent() {
        crate::util::paths::owner_only_dir(parent).context("create the schedule dir")?;
    }

    let temp = path.with_extension("partial");
    fs::write(&temp, day.format("%Y-%m-%d").to_string())
        .with_context(|| format!("write {}", temp.display()))?;

    fs::rename(&temp, path).with_context(|| format!("place {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(stamp: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(stamp, "%Y-%m-%d %H:%M").expect("parse the moment")
    }

    fn day(stamp: &str) -> NaiveDate {
        NaiveDate::parse_from_str(stamp, "%Y-%m-%d").expect("parse the day")
    }

    fn two_am() -> NaiveTime {
        NaiveTime::from_hms_opt(2, 0, 0).expect("02:00")
    }

    // --- Reading the target ------------------------------------------------

    #[test]
    fn a_target_time_is_read_from_the_clock_face() {
        for (raw, h, m) in [
            ("02:00", 2, 0),
            ("2:00", 2, 0),
            ("23:59", 23, 59),
            ("00:00", 0, 0),
            (" 02:30 ", 2, 30),
        ] {
            assert_eq!(
                parse_time_of_day(raw),
                NaiveTime::from_hms_opt(h, m, 0),
                "{raw:?}"
            );
        }
    }

    #[test]
    fn a_target_that_is_not_a_time_is_refused() {
        // The caller falls back to the interval, which delivers more often
        // rather than less. A typo must not be the reason a night is missed.
        for raw in ["", "2am", "24:00", "02:60", "02", "02:00:00", "-1:00", "two"] {
            assert_eq!(parse_time_of_day(raw), None, "{raw:?}");
        }
    }

    // --- Owing a delivery --------------------------------------------------

    #[test]
    fn a_schedule_that_has_never_delivered_is_owed_one_now() {
        // An installation upgrading into a schedule may hold a backlog, and
        // waiting a whole day for the first delivery would be a poor trade.
        assert!(is_due(at("2026-08-15 01:00"), two_am(), None));
    }

    #[test]
    fn nothing_is_owed_before_the_target_has_passed() {
        assert!(!is_due(
            at("2026-08-15 01:59"),
            two_am(),
            Some(day("2026-08-14"))
        ));
    }

    #[test]
    fn a_delivery_is_owed_once_the_target_passes_on_a_new_day() {
        assert!(is_due(
            at("2026-08-15 02:00"),
            two_am(),
            Some(day("2026-08-14"))
        ));
    }

    #[test]
    fn a_night_the_machine_slept_through_is_delivered_when_it_wakes() {
        // The whole reason this is held against the calendar. An interval only
        // fires while the application runs, so a machine asleep at 02:00 would
        // skip the night entirely.
        assert!(is_due(
            at("2026-08-15 09:30"),
            two_am(),
            Some(day("2026-08-13"))
        ));
    }

    #[test]
    fn a_day_already_delivered_is_not_delivered_again() {
        for moment in ["2026-08-15 02:00", "2026-08-15 13:00", "2026-08-15 23:59"] {
            assert!(
                !is_due(at(moment), two_am(), Some(day("2026-08-15"))),
                "{moment}"
            );
        }
    }

    #[test]
    fn a_clock_that_moved_backwards_still_delivers() {
        // Compared for difference rather than for being earlier. A marker
        // dated ahead of the clock would otherwise stop delivery until the
        // date caught up, which could be a very long night.
        assert!(is_due(
            at("2026-08-15 02:00"),
            two_am(),
            Some(day("2026-09-01"))
        ));
    }

    // --- The marker --------------------------------------------------------

    #[test]
    fn the_day_of_a_delivery_reads_back() {
        let dir = tempfile::TempDir::new().expect("temp dir");
        let path = dir.path().join("last.txt");

        record_delivery(&path, day("2026-08-15")).expect("record");

        assert_eq!(last_delivery(&path), Some(day("2026-08-15")));
    }

    #[test]
    fn a_marker_that_cannot_be_read_counts_as_no_marker() {
        // Erring towards delivering again costs a duplicate, which the sink
        // deduplicates on id. Erring the other way costs a night.
        let dir = tempfile::TempDir::new().expect("temp dir");

        let missing = dir.path().join("absent.txt");
        assert_eq!(last_delivery(&missing), None);

        let damaged = dir.path().join("damaged.txt");
        fs::write(&damaged, "not a date").expect("write junk");
        assert_eq!(last_delivery(&damaged), None);
        assert!(is_due(at("2026-08-15 09:00"), two_am(), last_delivery(&damaged)));
    }

    #[test]
    fn a_recorded_day_replaces_the_one_before_it() {
        let dir = tempfile::TempDir::new().expect("temp dir");
        let path = dir.path().join("last.txt");

        record_delivery(&path, day("2026-08-14")).expect("record");
        record_delivery(&path, day("2026-08-15")).expect("record again");

        assert_eq!(last_delivery(&path), Some(day("2026-08-15")));
    }
}

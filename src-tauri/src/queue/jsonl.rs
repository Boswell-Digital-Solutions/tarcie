use crate::constraints::{SENT_MAX_BYTES, SENT_RETENTION_DAYS};
use crate::model::TarcieEvent;
use crate::util::log;
use crate::util::paths::{owner_only_dir, owner_only_file, queue_dir, sent_dir};
use anyhow::{Context, Result};
use serde_json::Value;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

/// Distinguishes files stamped within the same second.
///
/// The stamp alone resolves to the second, so two rotations in one second
/// produced the same name and the second `rename` destroyed the first.
///
/// The count starts again when the process does. A run that begins in the
/// same second as an earlier one can therefore repeat a name that run already
/// used, which is what `free_path` exists to catch.
static SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// How many names to try before a placement is treated as impossible.
///
/// Names never repeat inside one run, so an attempt only fails when an earlier
/// run left the name behind. A crash leaves few files, and the limit is well
/// above that.
const MAX_NAME_ATTEMPTS: usize = 64;

fn stamp() -> String {
    let seq = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!(
        "{}-{:06}",
        chrono::Utc::now().format("%Y%m%dT%H%M%SZ"),
        seq % 1_000_000
    )
}

/// A path in `dir` that no file occupies yet.
///
/// `SEQUENCE` restarts with the process, so a run that begins in the same
/// second as a crashed one can build a name that is already on disk — an
/// orphan the crash left in `sending/`. `fs::rename` replaces such a file
/// without a word, and the events in it are then gone.
///
/// Each attempt takes a fresh stamp, so a retry sorts after the name it could
/// not have, and the claim order still follows the clock. An exhausted search
/// is an error: a failed flush leaves every event queued for the next one,
/// which the reliability contract prefers to an overwrite.
fn free_path(dir: &Path, name: impl Fn(&str) -> String) -> Result<PathBuf> {
    for _ in 0..MAX_NAME_ATTEMPTS {
        let candidate = dir.join(name(&stamp()));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }

    anyhow::bail!(
        "no free name in {} after {} attempts",
        dir.display(),
        MAX_NAME_ATTEMPTS
    )
}

/// Put a directory entry on the disk.
///
/// `File::sync_all` flushes a file's own contents and metadata. It says nothing
/// about the directory that names it. After a power loss the data can be on the
/// disk with no entry pointing at it, and for a queue file that means every
/// capture in it is gone. POSIX answers that by fsyncing the directory, which is
/// what this does.
///
/// Windows cannot open a directory as a file and has no equivalent call. Tarcie
/// qualifies on Linux first, so this is a no-op there rather than a failure.
#[cfg(unix)]
fn sync_dir(dir: &Path) -> Result<()> {
    File::open(dir)
        .with_context(|| format!("open {} to sync", dir.display()))?
        .sync_all()
        .with_context(|| format!("sync {}", dir.display()))
}

#[cfg(not(unix))]
fn sync_dir(_dir: &Path) -> Result<()> {
    Ok(())
}

/// Rename a file and put the new name on the disk.
///
/// Every rename in the queue is a handoff of custody, and a rename that has not
/// reached the disk is one a power loss can undo. Both directories are synced:
/// the one that gains the name, so the events are findable under it, and the one
/// that loses it, so the old name cannot come back and offer the same events a
/// second time.
///
/// The four callers are the same four that take their names through
/// `free_path` — claim, defer, archive, and cap rotation.
fn rename_durably(from: &Path, to: &Path) -> Result<()> {
    fs::rename(from, to)?;

    let gained = to.parent();
    if let Some(dir) = gained {
        sync_dir(dir)?;
    }

    if let Some(dir) = from.parent() {
        if Some(dir) != gained {
            sync_dir(dir)?;
        }
    }

    Ok(())
}

/// An archived batch, dated by the stamp in its own name.
struct ArchivedBatch {
    path: PathBuf,
    stamped_at: chrono::DateTime<chrono::Utc>,
    bytes: u64,
}

/// What bounding the archive cost, for the record.
///
/// Deleting a capture is the one thing tarcie does that a user cannot see and
/// cannot undo, so it does not happen quietly. The count, the size, and the
/// span go to the log; what the events said never does, which is the same rule
/// the rest of the log keeps.
#[derive(Default, PartialEq, Debug)]
struct Pruned {
    files: usize,
    bytes: u64,
    oldest: Option<chrono::DateTime<chrono::Utc>>,
    newest: Option<chrono::DateTime<chrono::Utc>>,
}

impl Pruned {
    fn record(&mut self, batch: &ArchivedBatch) {
        self.files += 1;
        self.bytes += batch.bytes;
        self.oldest.get_or_insert(batch.stamped_at);
        self.newest = Some(batch.stamped_at);
    }

    fn report(&self) {
        if self.files == 0 {
            return;
        }

        let span = match (self.oldest, self.newest) {
            (Some(o), Some(n)) => format!(
                "{} to {}",
                o.format("%Y-%m-%dT%H:%M:%SZ"),
                n.format("%Y-%m-%dT%H:%M:%SZ")
            ),
            _ => "unknown".to_string(),
        };

        log::write(format_args!(
            "archive bounded, dropped {} delivered batch(es), {} bytes, stamped {}",
            self.files, self.bytes, span
        ));
    }
}

/// The moment an archived batch was placed, read from its own name.
///
/// `queue.sent.{STAMP}.jsonl`, where the stamp is a UTC time to the second
/// followed by a sequence number. A name that does not take this shape returns
/// `None`, and its file is left alone.
fn stamped_at(path: &Path) -> Option<chrono::DateTime<chrono::Utc>> {
    let name = path.file_name()?.to_str()?;
    let rest = name.strip_prefix("queue.sent.")?.strip_suffix(".jsonl")?;
    let (moment, sequence) = rest.rsplit_once('-')?;

    if sequence.is_empty() || !sequence.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }

    chrono::NaiveDateTime::parse_from_str(moment, "%Y%m%dT%H%M%SZ")
        .ok()
        .map(|naive| naive.and_utc())
}

/// A batch of events taken out of the live queue and held for delivery.
///
/// Claiming moves the queue file aside, so events captured while a flush is in
/// flight land in a fresh queue file and are never mistaken for delivered.
pub struct Claim {
    files: Vec<PathBuf>,
    events: Vec<TarcieEvent>,
}

impl Claim {
    pub fn events(&self) -> &[TarcieEvent] {
        &self.events
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }
}

pub struct JsonlQueue {
    lock: Mutex<()>,
    queue_path: PathBuf,
    sending_dir: PathBuf,
    sent_path: PathBuf,
}

impl JsonlQueue {
    pub fn new() -> Result<Self> {
        Self::new_in(queue_dir()?, sent_dir()?)
    }

    /// Build a queue rooted at explicit directories.
    ///
    /// `new` resolves these from platform paths. This constructor takes them
    /// directly so a caller can isolate the queue from the real user profile.
    pub fn new_in(queue_dir: PathBuf, sent_dir: PathBuf) -> Result<Self> {
        owner_only_dir(&queue_dir).context("create queue dir")?;
        owner_only_dir(&sent_dir).context("create sent dir")?;
        let sending_dir = queue_dir.join("sending");
        owner_only_dir(&sending_dir).context("create sending dir")?;
        let queue_path = queue_dir.join("queue.jsonl");
        Ok(Self {
            lock: Mutex::new(()),
            queue_path,
            sending_dir,
            sent_path: sent_dir,
        })
    }

    pub fn append(&self, event: &TarcieEvent, queue_max_events: usize) -> Result<()> {
        let _g = self.lock.lock().unwrap();

        if self.line_count()? >= queue_max_events {
            self.rotate_at_cap_locked()?;
        }

        let json = serde_json::to_string(event).context("serialize event")?;
        let _: Value = serde_json::from_str(&json).context("sanity parse json")?;

        // Whether this append is the one that creates the file decides whether
        // the directory needs syncing after it.
        let creates_the_file = !self.queue_path.exists();

        let mut f = owner_only_file()
            .create(true)
            .append(true)
            .open(&self.queue_path)
            .context("open queue.jsonl for append")?;

        f.write_all(json.as_bytes()).context("write json")?;
        f.write_all(b"\n").context("write newline")?;
        f.flush().context("flush queue file")?;

        // Proper fsync for durability
        f.sync_all().context("fsync queue file")?;

        // The fsync above covers the contents. A file that has just been
        // created also needs the entry naming it on the disk, or a power loss
        // leaves the events with nothing pointing at them. Only the append that
        // creates the file pays for this, so the cost falls once per flush
        // cycle rather than once per capture.
        if creates_the_file {
            if let Some(dir) = self.queue_path.parent() {
                sync_dir(dir).context("sync the queue directory")?;
            }
        }

        Ok(())
    }

    pub fn read_all_tolerant(&self) -> Result<Vec<TarcieEvent>> {
        let _g = self.lock.lock().unwrap();
        read_tolerant(&self.queue_path)
    }

    /// Take everything currently queued, atomically.
    ///
    /// The live queue file is moved aside under the same lock that guards
    /// `append`, so an event captured during delivery lands in a fresh file and
    /// can never be archived as sent without being sent. A batch left behind by
    /// an interrupted flush is picked up here too, oldest first, so a crash
    /// costs a retry rather than a capture.
    pub fn claim(&self) -> Result<Claim> {
        let _g = self.lock.lock().unwrap();

        if self.queue_path.exists() {
            let target = free_path(&self.sending_dir, |s| format!("{s}.jsonl"))?;
            rename_durably(&self.queue_path, &target).context("claim the queue file")?;
        }

        let mut files = self.sending_files()?;
        files.sort();

        let mut events = Vec::new();
        for file in &files {
            events.extend(read_tolerant(file)?);
        }

        Ok(Claim { files, events })
    }

    /// Every event in the claim reached the sink.
    pub fn complete(&self, claim: Claim) -> Result<()> {
        let _g = self.lock.lock().unwrap();
        self.archive(claim.files)
    }

    /// Only the first `delivered` events of the claim reached the sink.
    ///
    /// The remainder is written back so the next flush retries what is still
    /// owed instead of resending a batch the sink already accepted.
    pub fn defer(&self, claim: Claim, delivered: usize) -> Result<()> {
        let _g = self.lock.lock().unwrap();

        if delivered == 0 {
            // Nothing was accepted, so the claim stands unchanged and the next
            // flush re-reads it as it is.
            return Ok(());
        }

        let remaining = &claim.events[delivered.min(claim.events.len())..];
        if !remaining.is_empty() {
            let target = free_path(&self.sending_dir, |s| format!("{s}.jsonl"))?;
            write_events(&target, remaining)?;
        }

        // The originals are archived rather than deleted. If the process dies
        // between writing the remainder and archiving them, the remainder is
        // delivered twice — which the reliability contract prefers to losing it.
        self.archive(claim.files)
    }

    fn archive(&self, files: Vec<PathBuf>) -> Result<()> {
        let placed = files.len();

        for file in files {
            let target = free_path(&self.sent_path, |s| format!("queue.sent.{s}.jsonl"))?;
            rename_durably(&file, &target).context("archive a delivered batch")?;
        }

        // Archiving is the only thing that grows the archive, so it is where
        // the archive is bounded. A claim that placed nothing skips it: this
        // runs under the lock that `append` waits on, and reading the whole
        // directory every flush cycle would put that in the capture path for
        // no reason. An idle run is caught by the pass at startup instead.
        if placed > 0 {
            self.prune_sent();
        }

        Ok(())
    }

    /// Bound the archive once, at startup.
    ///
    /// Without this, an installation that captures nothing never revisits what
    /// it already holds, and the retention period would go unkept on exactly
    /// the machines with the least reason to keep the archive at all.
    pub fn bound_archive(&self) {
        let _g = self.lock.lock().unwrap();
        self.prune_sent();
    }

    /// Bound the archive, and report what that cost.
    ///
    /// A delivered batch is kept for `SENT_RETENTION_DAYS`, and the whole
    /// archive is kept under `SENT_MAX_BYTES`. Whatever falls outside either
    /// bound is deleted, oldest first.
    fn prune_sent(&self) -> Pruned {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(SENT_RETENTION_DAYS);
        self.prune_sent_to(cutoff, SENT_MAX_BYTES)
    }

    /// The same bounds, chosen by the caller.
    ///
    /// `prune_sent` takes them from the constants and the clock. A test needs
    /// neither a ninety-day-old file nor a quarter of a gigabyte to prove the
    /// rule, so it passes its own.
    fn prune_sent_to(&self, cutoff: chrono::DateTime<chrono::Utc>, max_bytes: u64) -> Pruned {
        let mut batches = match self.archived_batches() {
            Ok(b) => b,
            Err(e) => {
                log::write(format_args!("could not read the archive to bound it: {e:#}"));
                return Pruned::default();
            }
        };

        // Oldest first, which is both the order they expire in and the order
        // they are given up in when the archive is too large.
        batches.sort_by_key(|b| b.stamped_at);

        let mut total: u64 = batches.iter().map(|b| b.bytes).sum();
        let mut pruned = Pruned::default();

        for batch in &batches {
            let too_old = batch.stamped_at < cutoff;
            let too_large = total > max_bytes;

            if !too_old && !too_large {
                break;
            }

            match fs::remove_file(&batch.path) {
                Ok(()) => {
                    total -= batch.bytes;
                    pruned.record(batch);
                }
                Err(e) => log::write(format_args!(
                    "could not drop an archived batch: {e}"
                )),
            }
        }

        pruned.report();
        pruned
    }

    /// Every archived batch this run can date, with its size.
    ///
    /// A file whose name does not carry a stamp this can read is left out, and
    /// so is never deleted. Nothing else writes to the archive, so such a file
    /// arrived by hand — and deleting what it cannot date is not something a
    /// capture tool should do.
    fn archived_batches(&self) -> Result<Vec<ArchivedBatch>> {
        let mut out = Vec::new();

        for entry in fs::read_dir(&self.sent_path).context("read the sent dir")? {
            let entry = entry.context("read a sent dir entry")?;
            let path = entry.path();

            let Some(stamped_at) = stamped_at(&path) else {
                continue;
            };
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            if !meta.is_file() {
                continue;
            }

            out.push(ArchivedBatch { path, stamped_at, bytes: meta.len() });
        }

        Ok(out)
    }

    fn sending_files(&self) -> Result<Vec<PathBuf>> {
        let mut out = Vec::new();
        for entry in fs::read_dir(&self.sending_dir).context("read the sending dir")? {
            let path = entry.context("read a sending dir entry")?.path();
            if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                out.push(path);
            }
        }
        Ok(out)
    }

    /// Move the live queue aside once it has reached the cap.
    ///
    /// The capped file goes into `sending/`, where the next claim picks it up
    /// and delivers it. It used to go into the sent directory, which no claim
    /// reads, so every event in it was discarded without a word — in the one
    /// situation the durable queue exists for, a sink that has been
    /// unreachable for a long time.
    ///
    /// The name keeps the stamp first, so a capped batch still sorts into
    /// place by age among the claimed ones.
    fn rotate_at_cap_locked(&self) -> Result<()> {
        if !self.queue_path.exists() {
            return Ok(());
        }

        let target = free_path(&self.sending_dir, |s| format!("{s}.cap.jsonl"))?;
        rename_durably(&self.queue_path, &target).context("rotate the capped queue file")?;

        // Reaching the cap means delivery has been failing for a long time.
        // Nothing else reports that.
        log::write(format_args!(
            "queue reached its cap, moved to {} to await delivery",
            target.display()
        ));

        Ok(())
    }

    fn line_count(&self) -> Result<usize> {
        if !self.queue_path.exists() {
            return Ok(0);
        }
        let f = File::open(&self.queue_path)?;
        let reader = BufReader::new(f);
        Ok(reader.lines().count())
    }
}

/// Read a JSONL file, skipping anything that does not parse.
///
/// A missing file reads as empty: a queue that was never written is not an
/// error, and neither is one a flush has just claimed.
fn read_tolerant(path: &Path) -> Result<Vec<TarcieEvent>> {
    if !path.exists() {
        return Ok(vec![]);
    }

    let f = File::open(path).with_context(|| format!("open {} for read", path.display()))?;
    let reader = BufReader::new(f);

    let mut out = Vec::new();
    for (idx, line) in reader.lines().enumerate() {
        let line = match line {
            Ok(s) => s,
            Err(e) => {
                log::write(format_args!("queue read error at line {}: {}", idx + 1, e));
                continue;
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<TarcieEvent>(&line) {
            Ok(ev) => out.push(ev),
            Err(e) => {
                log::write(format_args!("malformed json at line {}: {}", idx + 1, e));
                continue;
            }
        }
    }

    Ok(out)
}

/// Write events to a new file, durably.
///
/// The content is fsynced before the file is put in place, so a batch written
/// back after a partial delivery survives a crash immediately afterwards.
fn write_events(path: &Path, events: &[TarcieEvent]) -> Result<()> {
    let temp = path.with_extension("partial");

    let mut f = owner_only_file()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&temp)
        .context("open the deferred batch for write")?;

    for event in events {
        let json = serde_json::to_string(event).context("serialize a deferred event")?;
        f.write_all(json.as_bytes()).context("write a deferred event")?;
        f.write_all(b"\n").context("write newline")?;
    }

    f.flush().context("flush the deferred batch")?;
    f.sync_all().context("fsync the deferred batch")?;
    drop(f);

    rename_durably(&temp, path).context("put the deferred batch in place")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{EventType, TarcieEvent};
    use std::fs::OpenOptions;
    use tempfile::TempDir;
    use uuid::Uuid;

    /// A queue rooted in a fresh temp directory.
    ///
    /// The `TempDir` comes back with it because dropping the handle deletes
    /// the tree; a test must hold it for as long as it uses the queue.
    fn temp_queue() -> (JsonlQueue, TempDir) {
        let dir = TempDir::new().expect("create temp dir");
        let queue = JsonlQueue::new_in(dir.path().join("queue"), dir.path().join("queue/sent"))
            .expect("build queue");
        (queue, dir)
    }

    fn note(content: &str) -> TarcieEvent {
        TarcieEvent {
            id: Uuid::new_v4(),
            device_id: Uuid::nil(),
            timestamp_utc: chrono::Utc::now(),
            timestamp_mono_ms: 0,
            event_type: EventType::Note,
            content: content.to_string(),
            app_context: "General".to_string(),
            source_version: "test".to_string(),
        }
    }

    /// Files the queue has rotated away, oldest name first.
    fn rotated(queue: &JsonlQueue) -> Vec<PathBuf> {
        let mut paths: Vec<PathBuf> = fs::read_dir(&queue.sent_path)
            .expect("read sent dir")
            .map(|entry| entry.expect("dir entry").path())
            .collect();
        paths.sort();
        paths
    }

    fn append_raw(queue: &JsonlQueue, bytes: &[u8]) {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&queue.queue_path)
            .expect("open queue for raw append");
        file.write_all(bytes).expect("write raw bytes");
    }

    // --- 1. Append and read round-trip -----------------------------------

    #[test]
    fn append_then_read_returns_the_same_event() {
        let (queue, _dir) = temp_queue();
        let event = note("hello");

        queue.append(&event, 1000).expect("append");
        let read = queue.read_all_tolerant().expect("read");

        assert_eq!(read.len(), 1);
        assert_eq!(read[0].id, event.id);
        assert_eq!(read[0].content, "hello");
        assert_eq!(read[0].app_context, "General");
        assert!(matches!(read[0].event_type, EventType::Note));
    }

    #[test]
    fn append_preserves_order() {
        let (queue, _dir) = temp_queue();
        for i in 0..5 {
            queue.append(&note(&format!("n{i}")), 1000).expect("append");
        }

        let read = queue.read_all_tolerant().expect("read");
        let contents: Vec<&str> = read.iter().map(|e| e.content.as_str()).collect();
        assert_eq!(contents, ["n0", "n1", "n2", "n3", "n4"]);
    }

    #[test]
    fn marker_reason_survives_the_round_trip() {
        let (queue, _dir) = temp_queue();
        let mut event = note("");
        event.event_type = EventType::Marker { reason: Some("deploy".to_string()) };

        queue.append(&event, 1000).expect("append");
        let read = queue.read_all_tolerant().expect("read");

        match &read[0].event_type {
            EventType::Marker { reason } => assert_eq!(reason.as_deref(), Some("deploy")),
            other => panic!("expected a marker, got {other:?}"),
        }
    }

    #[test]
    fn reading_a_queue_that_was_never_written_is_empty_not_an_error() {
        let (queue, _dir) = temp_queue();
        assert!(queue.read_all_tolerant().expect("read").is_empty());
    }

    // --- 2. Tolerant read -------------------------------------------------

    #[test]
    fn a_malformed_line_is_skipped_and_the_valid_events_survive() {
        let (queue, _dir) = temp_queue();
        queue.append(&note("before"), 1000).expect("append");
        append_raw(&queue, b"{not valid json\n");
        queue.append(&note("after"), 1000).expect("append");

        let read = queue.read_all_tolerant().expect("read");
        let contents: Vec<&str> = read.iter().map(|e| e.content.as_str()).collect();
        assert_eq!(contents, ["before", "after"]);
    }

    #[test]
    fn a_truncated_final_line_does_not_lose_earlier_events() {
        let (queue, _dir) = temp_queue();
        queue.append(&note("durable"), 1000).expect("append");
        // A crash mid-write leaves a partial record with no trailing newline.
        append_raw(&queue, b"{\"id\":\"incomplete\"");

        let read = queue.read_all_tolerant().expect("read");
        assert_eq!(read.len(), 1);
        assert_eq!(read[0].content, "durable");
    }

    #[test]
    fn blank_lines_are_ignored() {
        let (queue, _dir) = temp_queue();
        queue.append(&note("kept"), 1000).expect("append");
        append_raw(&queue, b"\n   \n\n");

        assert_eq!(queue.read_all_tolerant().expect("read").len(), 1);
    }

    #[test]
    fn well_formed_json_that_is_not_an_event_is_skipped() {
        let (queue, _dir) = temp_queue();
        append_raw(&queue, b"{\"unrelated\":true}\n");
        queue.append(&note("kept"), 1000).expect("append");

        let read = queue.read_all_tolerant().expect("read");
        assert_eq!(read.len(), 1);
        assert_eq!(read[0].content, "kept");
    }

    // --- 3. Cap rotation --------------------------------------------------

    #[test]
    fn appending_at_the_cap_rotates_before_writing() {
        let (queue, _dir) = temp_queue();
        let cap = 4;
        for i in 0..cap {
            queue.append(&note(&format!("n{i}")), cap).expect("append");
        }
        assert_eq!(queue.read_all_tolerant().expect("read").len(), cap);

        queue.append(&note("overflow"), cap).expect("append past cap");

        let live = queue.read_all_tolerant().expect("read");
        assert_eq!(live.len(), 1, "the live queue restarts after rotation");
        assert_eq!(live[0].content, "overflow");

        assert!(
            rotated(&queue).is_empty(),
            "a capped batch was never sent, so nothing belongs in the sent dir"
        );

        let pending = queue.sending_files().expect("read the sending dir");
        assert_eq!(pending.len(), 1, "the capped batch waits for delivery");
        assert!(
            pending[0]
                .file_name()
                .unwrap()
                .to_string_lossy()
                .ends_with(".cap.jsonl"),
            "cap rotation is labelled distinctly from a claim"
        );
    }

    #[test]
    fn a_cap_rotation_retains_every_capped_event() {
        let (queue, _dir) = temp_queue();
        let cap = 4;
        for i in 0..=cap {
            queue.append(&note(&format!("n{i}")), cap).expect("append");
        }

        let pending = queue.sending_files().expect("read the sending dir");
        let moved = fs::read_to_string(&pending[0]).expect("read the capped file");
        assert_eq!(
            moved.lines().filter(|l| !l.trim().is_empty()).count(),
            cap,
            "rotation moves the capped events aside rather than discarding them"
        );
    }

    #[test]
    fn a_capped_batch_is_still_delivered() {
        // The cap is reached when delivery has been failing for a long time,
        // which is the one situation the durable queue exists for.
        let (queue, _dir) = temp_queue();
        let cap = 4;
        for i in 0..cap {
            queue.append(&note(&format!("n{i}")), cap).expect("append");
        }
        queue.append(&note("overflow"), cap).expect("append past cap");

        let claim = queue.claim().expect("claim");
        let contents: Vec<String> = claim.events().iter().map(|e| e.content.clone()).collect();

        assert_eq!(
            contents,
            ["n0", "n1", "n2", "n3", "overflow"],
            "every capped event is offered to the sink, oldest first"
        );
    }

    #[test]
    fn staying_under_the_cap_never_rotates() {
        let (queue, _dir) = temp_queue();
        for i in 0..3 {
            queue.append(&note(&format!("n{i}")), 100).expect("append");
        }
        assert!(rotated(&queue).is_empty());
        assert!(
            queue.sending_files().expect("read the sending dir").is_empty(),
            "nothing moves aside below the cap"
        );
    }

    // --- Claim, complete, defer ------------------------------------------

    #[test]
    fn completing_a_claim_clears_the_live_queue() {
        let (queue, _dir) = temp_queue();
        queue.append(&note("sent"), 1000).expect("append");

        let claim = queue.claim().expect("claim");
        assert_eq!(claim.len(), 1);
        queue.complete(claim).expect("complete");

        assert!(queue.read_all_tolerant().expect("read").is_empty());
        let rotated = rotated(&queue);
        assert_eq!(rotated.len(), 1);
        assert!(rotated[0].file_name().unwrap().to_string_lossy().starts_with("queue.sent."));
    }

    #[test]
    fn claiming_an_empty_queue_yields_an_empty_claim() {
        let (queue, _dir) = temp_queue();
        let claim = queue.claim().expect("claim");
        assert!(claim.is_empty());
        queue.complete(claim).expect("complete");
        assert!(rotated(&queue).is_empty());
    }

    #[test]
    fn an_event_captured_during_a_flush_is_not_archived_as_sent() {
        // The defect this guards: the flusher read the queue, posted, then
        // rotated the file. Anything appended in between was filed as sent
        // without ever being sent. Claiming moves the file aside first, so the
        // late capture lands in a fresh queue and survives.
        let (queue, _dir) = temp_queue();
        queue.append(&note("in the batch"), 1000).expect("append");

        let claim = queue.claim().expect("claim");
        queue.append(&note("captured mid-flush"), 1000).expect("append during flush");
        queue.complete(claim).expect("complete");

        let survivors = queue.read_all_tolerant().expect("read");
        assert_eq!(survivors.len(), 1, "the late capture is still queued");
        assert_eq!(survivors[0].content, "captured mid-flush");
    }

    #[test]
    fn deferring_keeps_only_what_was_not_delivered() {
        // The defect this guards: a multi-batch flush that failed partway
        // deferred the whole claim, so batches the sink had already accepted
        // were posted again on the next cycle.
        let (queue, _dir) = temp_queue();
        for i in 0..5 {
            queue.append(&note(&format!("n{i}")), 1000).expect("append");
        }

        let claim = queue.claim().expect("claim");
        queue.defer(claim, 2).expect("defer after two delivered");

        let retry = queue.claim().expect("reclaim");
        let contents: Vec<&str> = retry.events().iter().map(|e| e.content.as_str()).collect();
        assert_eq!(contents, ["n2", "n3", "n4"], "only the undelivered remainder returns");
    }

    #[test]
    fn deferring_with_nothing_delivered_keeps_the_whole_claim() {
        let (queue, _dir) = temp_queue();
        for i in 0..3 {
            queue.append(&note(&format!("n{i}")), 1000).expect("append");
        }

        let claim = queue.claim().expect("claim");
        queue.defer(claim, 0).expect("defer with nothing delivered");

        let retry = queue.claim().expect("reclaim");
        assert_eq!(retry.len(), 3);
    }

    #[test]
    fn a_batch_left_by_an_interrupted_flush_is_reclaimed() {
        // A claim that is never completed or deferred — the process died mid
        // flush — must come back on the next claim rather than be stranded.
        let (queue, _dir) = temp_queue();
        queue.append(&note("orphaned"), 1000).expect("append");

        let claim = queue.claim().expect("claim");
        drop(claim); // the process dies here

        let recovered = queue.claim().expect("reclaim");
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered.events()[0].content, "orphaned");
    }

    #[test]
    fn a_reclaim_returns_recovered_events_before_newer_ones() {
        let (queue, _dir) = temp_queue();
        queue.append(&note("older"), 1000).expect("append");
        drop(queue.claim().expect("claim")); // stranded

        queue.append(&note("newer"), 1000).expect("append");

        let claim = queue.claim().expect("reclaim");
        let contents: Vec<&str> = claim.events().iter().map(|e| e.content.as_str()).collect();
        assert_eq!(contents, ["older", "newer"], "capture order survives recovery");
    }

    #[test]
    fn rotations_within_one_second_do_not_overwrite_each_other() {
        // The stamp resolves to the second, so a bare timestamp collided and
        // the second rename destroyed the first file.
        let (queue, _dir) = temp_queue();
        for i in 0..4 {
            queue.append(&note(&format!("n{i}")), 1000).expect("append");
            let claim = queue.claim().expect("claim");
            queue.complete(claim).expect("complete");
        }

        assert_eq!(rotated(&queue).len(), 4, "each rotation keeps its own file");
    }

    // --- Names a crashed run left behind ----------------------------------
    //
    // The sequence that separates same-second names counts from zero again
    // when the process starts. A run that begins in the second a crashed run
    // ended in can therefore build a name that is already on disk.

    #[test]
    fn a_name_a_crashed_run_left_behind_is_never_taken_over() {
        let dir = TempDir::new().expect("create temp dir");
        let orphan = dir.path().join("taken.jsonl");
        fs::write(&orphan, "an undelivered batch").expect("plant the orphan");

        // The first attempt offers the orphan's name, as a restarted sequence
        // would. The attempt after it offers a name nothing holds.
        let attempt = std::cell::Cell::new(0);
        let path = free_path(dir.path(), |_stamp| {
            let n = attempt.get();
            attempt.set(n + 1);
            if n == 0 {
                "taken.jsonl".to_string()
            } else {
                "fresh.jsonl".to_string()
            }
        })
        .expect("find a free name");

        assert_eq!(path, dir.path().join("fresh.jsonl"));
        assert_eq!(
            fs::read_to_string(&orphan).expect("read the orphan"),
            "an undelivered batch",
            "the batch the crashed run left is still there"
        );
    }

    #[test]
    fn a_free_name_is_taken_on_the_first_attempt() {
        let dir = TempDir::new().expect("create temp dir");

        let path = free_path(dir.path(), |s| format!("{s}.jsonl")).expect("find a free name");

        assert_eq!(path.parent(), Some(dir.path()));
        assert!(!path.exists(), "the name is free, and stays free until used");
    }

    #[test]
    fn a_search_that_never_finds_a_free_name_fails_instead_of_overwriting() {
        // Failing the placement leaves every event where it is, and the next
        // flush tries again. Overwriting would spend them.
        let dir = TempDir::new().expect("create temp dir");
        let orphan = dir.path().join("taken.jsonl");
        fs::write(&orphan, "an undelivered batch").expect("plant the orphan");

        let result = free_path(dir.path(), |_stamp| "taken.jsonl".to_string());

        assert!(result.is_err(), "an exhausted search is an error");
        assert_eq!(
            fs::read_to_string(&orphan).expect("read the orphan"),
            "an undelivered batch",
            "nothing was written over it"
        );
    }

    // --- Who can read a capture --------------------------------------------

    #[cfg(unix)]
    fn mode_of(path: &Path) -> u32 {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(path).expect("stat").permissions().mode() & 0o777
    }

    #[cfg(unix)]
    #[test]
    fn the_queue_is_closed_to_every_other_account() {
        // The files hold the user's notes verbatim and nothing masks them.
        // Left to the umask they are created 0644 in directories at 0755, so
        // whether anyone else with a login can read them is the
        // distribution's choice about the home directory, not tarcie's.
        let (queue, _dir) = temp_queue();
        queue.append(&note("private"), 1000).expect("append");

        assert_eq!(mode_of(&queue.queue_path), 0o600, "the queue file");

        for dir in [
            queue.queue_path.parent().expect("queue dir"),
            &queue.sending_dir,
            &queue.sent_path,
        ] {
            assert_eq!(mode_of(dir), 0o700, "{}", dir.display());
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_directory_an_earlier_version_left_open_is_closed() {
        // Creation does not touch a directory that is already there, so an
        // installation that predates this would keep 0755 forever.
        use std::os::unix::fs::PermissionsExt;

        let dir = TempDir::new().expect("create temp dir");
        let open = dir.path().join("queue");
        fs::create_dir_all(&open).expect("create it open");
        fs::set_permissions(&open, fs::Permissions::from_mode(0o755)).expect("open it up");

        crate::util::paths::owner_only_dir(&open).expect("close it");

        assert_eq!(mode_of(&open), 0o700);
    }

    #[cfg(unix)]
    #[test]
    fn a_deferred_batch_is_written_closed() {
        // The remainder of a partial delivery is a fresh file holding events
        // the sink has not taken, so it is created like any other.
        let dir = TempDir::new().expect("create temp dir");
        let path = dir.path().join("remainder.jsonl");

        write_events(&path, &[note("still owed")]).expect("write the remainder");

        assert_eq!(mode_of(&path), 0o600);
    }

    // --- Bounding the archive ----------------------------------------------
    //
    // The archive kept every capture ever made, for the life of the
    // installation. Nothing in tarcie reads it, so these bounds decide what a
    // forensic copy is worth keeping.

    /// An archived batch of `bytes` bytes, stamped at `moment`.
    fn archived(queue: &JsonlQueue, moment: &str, bytes: usize) -> PathBuf {
        let path = queue
            .sent_path
            .join(format!("queue.sent.{moment}-000000.jsonl"));
        fs::write(&path, "x".repeat(bytes)).expect("write an archived batch");
        path
    }

    fn day(stamp: &str) -> chrono::DateTime<chrono::Utc> {
        chrono::NaiveDateTime::parse_from_str(stamp, "%Y%m%dT%H%M%SZ")
            .expect("parse the stamp")
            .and_utc()
    }

    #[test]
    fn a_batch_older_than_the_period_is_dropped_and_a_newer_one_is_kept() {
        let (queue, _dir) = temp_queue();
        let old = archived(&queue, "20250101T000000Z", 10);
        let recent = archived(&queue, "20260814T000000Z", 10);

        let pruned = queue.prune_sent_to(day("20260101T000000Z"), u64::MAX);

        assert!(!old.exists(), "the batch outside the period is gone");
        assert!(recent.exists(), "the batch inside it is kept");
        assert_eq!(pruned.files, 1);
        assert_eq!(pruned.bytes, 10);
    }

    #[test]
    fn an_archive_over_its_ceiling_gives_up_its_oldest_first() {
        let (queue, _dir) = temp_queue();
        let oldest = archived(&queue, "20260101T000000Z", 100);
        let middle = archived(&queue, "20260601T000000Z", 100);
        let newest = archived(&queue, "20260801T000000Z", 100);

        // Everything is inside the period, so only the ceiling is at work.
        let pruned = queue.prune_sent_to(day("20200101T000000Z"), 250);

        assert!(!oldest.exists(), "the oldest goes first");
        assert!(middle.exists(), "and no more than the ceiling asks for");
        assert!(newest.exists());
        assert_eq!(pruned.files, 1);
    }

    #[test]
    fn an_archive_inside_both_bounds_is_left_alone() {
        let (queue, _dir) = temp_queue();
        let kept = archived(&queue, "20260801T000000Z", 100);

        let pruned = queue.prune_sent_to(day("20200101T000000Z"), u64::MAX);

        assert!(kept.exists());
        assert_eq!(pruned, Pruned::default(), "nothing was dropped, and nothing reported");
    }

    #[test]
    fn a_file_the_archive_cannot_date_is_never_deleted() {
        // Nothing but `archive` writes here, so a name of another shape
        // arrived by hand. Deleting what it cannot date is not a capture
        // tool's business.
        let (queue, _dir) = temp_queue();

        let strangers = ["notes-i-copied-here.jsonl", "queue.sent.jsonl", "queue.sent.later-000000.jsonl"];
        for name in strangers {
            fs::write(queue.sent_path.join(name), "x").expect("plant a stranger");
        }

        let pruned = queue.prune_sent_to(day("20990101T000000Z"), 0);

        for name in strangers {
            assert!(
                queue.sent_path.join(name).exists(),
                "{name} was not tarcie's to delete"
            );
        }
        assert_eq!(pruned.files, 0);
    }

    #[test]
    fn bounding_the_archive_reports_what_it_cost() {
        // Deleting a capture is the one thing tarcie does that a user cannot
        // see and cannot undo, so the log carries the count, the size, and the
        // span — and never what the events said.
        let (queue, _dir) = temp_queue();
        archived(&queue, "20250101T000000Z", 40);
        archived(&queue, "20250201T000000Z", 60);

        let pruned = queue.prune_sent_to(day("20260101T000000Z"), u64::MAX);

        assert_eq!(pruned.files, 2);
        assert_eq!(pruned.bytes, 100);
        assert_eq!(pruned.oldest, Some(day("20250101T000000Z")));
        assert_eq!(pruned.newest, Some(day("20250201T000000Z")));
    }

    #[test]
    fn a_delivered_batch_is_bounded_when_it_is_archived() {
        // The bound is applied where the archive grows, so no separate step
        // has to remember to run.
        let (queue, _dir) = temp_queue();
        let ancient = archived(&queue, "20200101T000000Z", 10);

        queue.append(&note("deliver me"), 1000).expect("append");
        let claim = queue.claim().expect("claim");
        queue.complete(claim).expect("complete");

        assert!(
            !ancient.exists(),
            "archiving a delivered batch bounds what is already there"
        );
        assert_eq!(rotated(&queue).len(), 1, "and the batch just delivered is kept");
    }

    #[test]
    fn a_run_that_archives_nothing_still_keeps_the_retention_period() {
        // The bound is applied where the archive grows, and a run that never
        // delivers never grows it. Without the pass at startup, the period
        // would go unkept on exactly the installations holding the most
        // forgotten captures.
        let (queue, _dir) = temp_queue();
        let ancient = queue
            .sent_path
            .join("queue.sent.20200101T000000Z-000000.jsonl");
        fs::write(&ancient, "an old delivered batch").expect("plant it");

        queue.bound_archive();

        assert!(!ancient.exists());
    }

    // --- Durable placement -------------------------------------------------
    //
    // A rename that has not reached the disk is one a power loss can undo.
    // Whether it survives a power loss is not testable here; that these two
    // syncs cost nothing in behaviour is.

    #[test]
    fn a_durable_placement_moves_the_file_and_syncs_both_directories() {
        let dir = TempDir::new().expect("create temp dir");
        let from_dir = dir.path().join("from");
        let to_dir = dir.path().join("to");
        fs::create_dir_all(&from_dir).expect("create the source dir");
        fs::create_dir_all(&to_dir).expect("create the target dir");

        let source = from_dir.join("batch.jsonl");
        let target = to_dir.join("batch.jsonl");
        fs::write(&source, "a batch").expect("write the batch");

        // A directory either sync cannot open would fail here rather than be
        // skipped without a word.
        rename_durably(&source, &target).expect("place the batch durably");

        assert!(!source.exists(), "the old name is gone");
        assert_eq!(
            fs::read_to_string(&target).expect("read the batch"),
            "a batch"
        );
    }

    #[test]
    fn a_placement_that_cannot_happen_leaves_the_events_where_they_were() {
        // The same ground `free_path` holds: a placement that cannot happen
        // costs a retry and never a capture.
        let dir = TempDir::new().expect("create temp dir");
        let source = dir.path().join("batch.jsonl");
        fs::write(&source, "an undelivered batch").expect("write the batch");

        let result = rename_durably(&source, &dir.path().join("gone").join("batch.jsonl"));

        assert!(result.is_err(), "a placement into a missing directory fails");
        assert_eq!(
            fs::read_to_string(&source).expect("read the batch"),
            "an undelivered batch",
            "the batch is still where the next flush will find it"
        );
    }

    #[test]
    fn a_claim_keeps_what_an_earlier_run_left_in_sending() {
        // A crash between the claim and the archive leaves files in sending/.
        // The next run claims them alongside the live queue, so the events are
        // offered again rather than stranded.
        let (queue, dir) = temp_queue();
        let orphan = dir.path().join("queue/sending/20200101T000000Z-000000.jsonl");
        write_events(&orphan, &[note("from the crashed run")]).expect("plant the orphan");

        queue.append(&note("from this run"), 1000).expect("append");

        let claim = queue.claim().expect("claim");
        let contents: Vec<String> = claim.events().iter().map(|e| e.content.clone()).collect();

        assert_eq!(
            contents,
            ["from the crashed run", "from this run"],
            "the older batch is delivered first, and neither is lost"
        );
    }
}

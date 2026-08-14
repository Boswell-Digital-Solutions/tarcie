use crate::model::TarcieEvent;
use crate::util::paths::{queue_dir, sent_dir};
use anyhow::{Context, Result};
use serde_json::Value;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

/// Distinguishes files stamped within the same second.
///
/// The stamp alone resolves to the second, so two rotations in one second
/// produced the same name and the second `rename` destroyed the first.
static SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn stamp() -> String {
    let seq = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!(
        "{}-{:06}",
        chrono::Utc::now().format("%Y%m%dT%H%M%SZ"),
        seq % 1_000_000
    )
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
        fs::create_dir_all(&queue_dir).context("create queue dir")?;
        fs::create_dir_all(&sent_dir).context("create sent dir")?;
        let sending_dir = queue_dir.join("sending");
        fs::create_dir_all(&sending_dir).context("create sending dir")?;
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
            self.rotate_locked("queue.cap")?;
        }

        let json = serde_json::to_string(event).context("serialize event")?;
        let _: Value = serde_json::from_str(&json).context("sanity parse json")?;

        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.queue_path)
            .context("open queue.jsonl for append")?;

        f.write_all(json.as_bytes()).context("write json")?;
        f.write_all(b"\n").context("write newline")?;
        f.flush().context("flush queue file")?;

        // Proper fsync for durability
        f.sync_all().context("fsync queue file")?;

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
            let target = self.sending_dir.join(format!("{}.jsonl", stamp()));
            fs::rename(&self.queue_path, &target).context("claim the queue file")?;
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
            let target = self.sending_dir.join(format!("{}.jsonl", stamp()));
            write_events(&target, remaining)?;
        }

        // The originals are archived rather than deleted. If the process dies
        // between writing the remainder and archiving them, the remainder is
        // delivered twice — which the reliability contract prefers to losing it.
        self.archive(claim.files)
    }

    fn archive(&self, files: Vec<PathBuf>) -> Result<()> {
        for file in files {
            let target = self
                .sent_path
                .join(format!("queue.sent.{}.jsonl", stamp()));
            fs::rename(&file, &target).context("archive a delivered batch")?;
        }
        Ok(())
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

    fn rotate_locked(&self, prefix: &str) -> Result<()> {
        if !self.queue_path.exists() {
            return Ok(());
        }
        let sent = self
            .sent_path
            .join(format!("{}.{}.jsonl", prefix, stamp()));
        fs::rename(&self.queue_path, &sent).context("rotate queue file")?;
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
                eprintln!("tarcie: queue read error at line {}: {}", idx + 1, e);
                continue;
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<TarcieEvent>(&line) {
            Ok(ev) => out.push(ev),
            Err(e) => {
                eprintln!("tarcie: malformed json at line {}: {}", idx + 1, e);
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

    let mut f = OpenOptions::new()
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

    fs::rename(&temp, path).context("put the deferred batch in place")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{EventType, TarcieEvent};
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

        let rotated = rotated(&queue);
        assert_eq!(rotated.len(), 1);
        assert!(
            rotated[0].file_name().unwrap().to_string_lossy().starts_with("queue.cap."),
            "cap rotation is labelled distinctly from a successful flush"
        );
    }

    #[test]
    fn a_cap_rotation_retains_every_capped_event() {
        let (queue, _dir) = temp_queue();
        let cap = 4;
        for i in 0..=cap {
            queue.append(&note(&format!("n{i}")), cap).expect("append");
        }

        let archived = fs::read_to_string(&rotated(&queue)[0]).expect("read rotated file");
        assert_eq!(
            archived.lines().filter(|l| !l.trim().is_empty()).count(),
            cap,
            "rotation moves the capped events aside rather than discarding them"
        );
    }

    #[test]
    fn staying_under_the_cap_never_rotates() {
        let (queue, _dir) = temp_queue();
        for i in 0..3 {
            queue.append(&note(&format!("n{i}")), 100).expect("append");
        }
        assert!(rotated(&queue).is_empty());
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
}

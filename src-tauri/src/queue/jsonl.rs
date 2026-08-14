use crate::model::TarcieEvent;
use crate::util::paths::{queue_dir, sent_dir};
use anyhow::{Context, Result};
use serde_json::Value;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::Mutex;

pub struct JsonlQueue {
    lock: Mutex<()>,
    queue_path: PathBuf,
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
        let queue_path = queue_dir.join("queue.jsonl");
        Ok(Self { lock: Mutex::new(()), queue_path, sent_path: sent_dir })
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

        if !self.queue_path.exists() {
            return Ok(vec![]);
        }

        let f = File::open(&self.queue_path).context("open queue.jsonl for read")?;
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

    pub fn rotate_on_success(&self) -> Result<()> {
        let _g = self.lock.lock().unwrap();
        self.rotate_locked("queue.sent")
    }

    fn rotate_locked(&self, prefix: &str) -> Result<()> {
        if !self.queue_path.exists() {
            return Ok(());
        }
        let ts = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
        let sent = self.sent_path.join(format!("{}.{}.jsonl", prefix, ts));
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

    #[test]
    fn rotate_on_success_clears_the_live_queue() {
        let (queue, _dir) = temp_queue();
        queue.append(&note("sent"), 1000).expect("append");

        queue.rotate_on_success().expect("rotate");

        assert!(queue.read_all_tolerant().expect("read").is_empty());
        let rotated = rotated(&queue);
        assert_eq!(rotated.len(), 1);
        assert!(rotated[0].file_name().unwrap().to_string_lossy().starts_with("queue.sent."));
    }

    #[test]
    fn rotating_with_no_queue_file_is_a_no_op() {
        let (queue, _dir) = temp_queue();
        queue.rotate_on_success().expect("rotate on empty queue");
        assert!(rotated(&queue).is_empty());
    }
}

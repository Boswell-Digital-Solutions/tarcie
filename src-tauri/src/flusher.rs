use crate::model::TarcieEvent;
use crate::queue::jsonl::JsonlQueue;
use crate::sink::client::SinkClient;
use crate::sink::config::SinkConfig;
use anyhow::Result;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

#[derive(Serialize)]
struct IngestPayload<'a> {
    source: &'static str,
    events: &'a [TarcieEvent],
}

pub struct Flusher {
    queue: Arc<JsonlQueue>,
    sink: SinkClient,
    cfg: SinkConfig,
    lock: Mutex<()>,
}

pub enum FlushResult {
    Empty,
    Success { count: usize },
    Deferred { reason: String },
}

impl Flusher {
    pub fn new(queue: Arc<JsonlQueue>, sink: SinkClient, cfg: SinkConfig) -> Self {
        Self { queue, sink, cfg, lock: Mutex::new(()) }
    }

    pub async fn flush_with_retry(&self) -> Result<FlushResult> {
        let _g = self.lock.lock().await;

        // Claiming moves the queue aside before anything is posted, so an
        // event captured during delivery lands in a fresh queue file instead
        // of being archived as sent without ever being sent.
        let claim = self.queue.claim()?;
        if claim.is_empty() {
            // The claim may still hold files whose every line was unparsable.
            // Archiving them keeps the next flush from claiming them forever.
            self.queue.complete(claim)?;
            return Ok(FlushResult::Empty);
        }

        let total = claim.len();
        let batch_max = self.cfg.batch_max;
        let mut delivered = 0usize;
        let mut failure: Option<String> = None;

        {
            let events = claim.events();
            for chunk in events.chunks(batch_max) {
                match self.post_chunk(chunk).await {
                    Ok(()) => delivered += chunk.len(),
                    Err(e) => {
                        failure = Some(e.to_string());
                        break;
                    }
                }
            }
        }

        match failure {
            // The batches that were accepted are not offered again; only what
            // is still owed goes back for the next cycle.
            Some(reason) => {
                self.queue.defer(claim, delivered)?;
                Ok(FlushResult::Deferred { reason })
            }
            None => {
                self.queue.complete(claim)?;
                Ok(FlushResult::Success { count: total })
            }
        }
    }

    async fn post_chunk(&self, chunk: &[TarcieEvent]) -> Result<()> {
        let payload = IngestPayload { source: "tarcie", events: chunk };

        let mut retries = 0u32;
        loop {
            match self.sink.post_json(&payload).await {
                Ok(_) => return Ok(()),
                Err(_) if retries < 3 => {
                    retries += 1;
                    let backoff = 2u64.pow(retries);
                    sleep(Duration::from_secs(backoff)).await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    pub fn cfg(&self) -> &SinkConfig {
        &self.cfg
    }

    pub fn queue(&self) -> &JsonlQueue {
        &self.queue
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::EventType;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::thread::{self, JoinHandle};
    use tempfile::TempDir;
    use uuid::Uuid;

    /// Loopback port 1 is privileged and unbound, so a connection to it is
    /// refused immediately rather than hanging.
    const UNREACHABLE: &str = "http://127.0.0.1:1/ingest";

    fn temp_queue() -> (Arc<JsonlQueue>, TempDir) {
        let dir = TempDir::new().expect("create temp dir");
        let queue = JsonlQueue::new_in(dir.path().join("queue"), dir.path().join("queue/sent"))
            .expect("build queue");
        (Arc::new(queue), dir)
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

    fn flusher_for(url: &str, queue: Arc<JsonlQueue>) -> Flusher {
        flusher_with_batch(url, queue, None)
    }

    fn flusher_with_batch(url: &str, queue: Arc<JsonlQueue>, batch_max: Option<usize>) -> Flusher {
        let url = url.to_string();
        let batch = batch_max.map(|n| n.to_string());
        let cfg = SinkConfig::resolve(|key| match key {
            "TARCIE_SINK_URL" => Some(url.clone()),
            "TARCIE_BATCH_MAX" => batch.clone(),
            _ => None,
        })
        .expect("resolve sink config");

        let sink = SinkClient::new(cfg.url.clone(), cfg.auth.clone()).expect("build sink client");
        Flusher::new(queue, sink, cfg)
    }

    /// What the queue still owes, taken through a fresh claim.
    fn still_owed(queue: &JsonlQueue) -> Vec<String> {
        queue
            .claim()
            .expect("claim")
            .events()
            .iter()
            .map(|e| e.content.clone())
            .collect()
    }

    fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack.windows(needle.len()).position(|w| w == needle)
    }

    /// Consume one whole HTTP request, headers and declared body both, so the
    /// client sees a complete exchange instead of a reset connection.
    fn read_request(stream: &mut TcpStream) -> Vec<u8> {
        let mut buf = Vec::new();
        let mut chunk = [0u8; 2048];
        loop {
            match stream.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(n) => buf.extend_from_slice(&chunk[..n]),
            }
            if let Some(head_end) = find(&buf, b"\r\n\r\n") {
                let head = String::from_utf8_lossy(&buf[..head_end]).to_lowercase();
                let body_len = head
                    .lines()
                    .find_map(|line| line.strip_prefix("content-length:"))
                    .and_then(|v| v.trim().parse::<usize>().ok())
                    .unwrap_or(0);
                if buf.len() >= head_end + 4 + body_len {
                    break;
                }
            }
        }
        buf
    }

    /// A one-shot HTTP endpoint on an ephemeral port. The handle joins to the
    /// request body it received, so a test can assert on what was sent.
    fn spawn_sink(response: &'static str) -> (String, JoinHandle<Vec<u8>>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let addr = listener.local_addr().expect("local addr");

        let handle = thread::spawn(move || {
            let mut received = Vec::new();
            if let Ok((mut stream, _)) = listener.accept() {
                received = read_request(&mut stream);
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
            }
            received
        });

        (format!("http://{addr}/ingest"), handle)
    }

    const OK: &str = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n";

    // --- 7. FlushResult variants ------------------------------------------

    #[tokio::test]
    async fn an_empty_queue_reports_empty_without_contacting_the_sink() {
        let (queue, _dir) = temp_queue();
        let flusher = flusher_for(UNREACHABLE, queue);

        // The sink is unreachable; reaching it at all would fail the test.
        assert!(matches!(
            flusher.flush_with_retry().await.expect("flush"),
            FlushResult::Empty
        ));
    }

    #[tokio::test]
    async fn a_successful_post_reports_the_count_and_clears_the_queue() {
        let (queue, _dir) = temp_queue();
        queue.append(&note("one"), 1000).expect("append");
        queue.append(&note("two"), 1000).expect("append");

        let (url, server) = spawn_sink(OK);
        let flusher = flusher_for(&url, Arc::clone(&queue));

        match flusher.flush_with_retry().await.expect("flush") {
            FlushResult::Success { count } => assert_eq!(count, 2),
            FlushResult::Empty => panic!("expected success, got empty"),
            FlushResult::Deferred { reason } => panic!("expected success, got deferral: {reason}"),
        }

        assert!(
            queue.read_all_tolerant().expect("read").is_empty(),
            "a successful flush rotates the queue away"
        );

        let body = server.join().expect("sink thread");
        let text = String::from_utf8_lossy(&body);
        assert!(text.contains("\"source\":\"tarcie\""), "payload names its source");
        assert!(text.contains("one") && text.contains("two"), "both events were sent");
    }

    #[tokio::test(start_paused = true)]
    async fn an_unreachable_sink_defers_and_keeps_every_event() {
        let (queue, _dir) = temp_queue();
        queue.append(&note("keep me"), 1000).expect("append");

        let flusher = flusher_for(UNREACHABLE, Arc::clone(&queue));

        match flusher.flush_with_retry().await.expect("flush") {
            FlushResult::Deferred { reason } => {
                assert!(!reason.is_empty(), "a deferral explains itself")
            }
            FlushResult::Empty => panic!("expected deferral, got empty"),
            FlushResult::Success { .. } => panic!("expected deferral, got success"),
        }

        // The reliability contract: an unreachable sink never costs a capture.
        assert_eq!(still_owed(&queue), ["keep me"]);
    }

    #[tokio::test(start_paused = true)]
    async fn a_sink_error_status_defers_and_keeps_every_event() {
        let (queue, _dir) = temp_queue();
        queue.append(&note("keep me"), 1000).expect("append");

        // One 500 response, then the listener closes; the retries that follow
        // hit a dead port and the flush gives up without dropping anything.
        let (url, server) = spawn_sink("HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\n\r\n");
        let flusher = flusher_for(&url, Arc::clone(&queue));

        let result = flusher.flush_with_retry().await.expect("flush");
        assert!(
            matches!(result, FlushResult::Deferred { .. }),
            "a rejecting sink defers rather than discarding"
        );

        assert_eq!(still_owed(&queue), ["keep me"]);

        let _ = server.join();
    }

    #[tokio::test(start_paused = true)]
    async fn a_partial_multi_batch_delivery_retries_only_what_is_owed() {
        // The sink accepts the first batch of two, then the listener closes and
        // the rest of the flush fails. The accepted batch must not be offered
        // again — resending it would duplicate what the sink already holds.
        let (queue, _dir) = temp_queue();
        for i in 0..4 {
            queue.append(&note(&format!("n{i}")), 1000).expect("append");
        }

        let (url, server) = spawn_sink(OK);
        let flusher = flusher_with_batch(&url, Arc::clone(&queue), Some(2));

        assert!(
            matches!(
                flusher.flush_with_retry().await.expect("flush"),
                FlushResult::Deferred { .. }
            ),
            "a failure partway through defers"
        );
        let _ = server.join();

        assert_eq!(still_owed(&queue), ["n2", "n3"]);
    }

    // The mid-flush capture window is proven at the queue layer, in
    // `queue::jsonl::tests::an_event_captured_during_a_flush_is_not_archived_as_sent`,
    // where the append can be placed between the claim and the completion. A
    // flusher-level test cannot reach inside `flush_with_retry` to do that, so
    // there is no honest end-to-end version of it here.
}

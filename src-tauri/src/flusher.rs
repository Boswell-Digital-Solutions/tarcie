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

        let events = self.queue.read_all_tolerant()?;
        if events.is_empty() {
            return Ok(FlushResult::Empty);
        }

        let batch_max = self.cfg.batch_max;
        for chunk in events.chunks(batch_max) {
            let payload = IngestPayload { source: "tarcie", events: chunk };

            let mut retries = 0u32;
            loop {
                match self.sink.post_json(&payload).await {
                    Ok(_) => break,
                    Err(_) if retries < 3 => {
                        retries += 1;
                        let backoff = 2u64.pow(retries);
                        sleep(Duration::from_secs(backoff)).await;
                        continue;
                    }
                    Err(e) => {
                        return Ok(FlushResult::Deferred { reason: e.to_string() });
                    }
                }
            }
        }

        self.queue.rotate_on_success()?;
        Ok(FlushResult::Success { count: events.len() })
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
        let url = url.to_string();
        let cfg = SinkConfig::resolve(|key| match key {
            "TARCIE_SINK_URL" => Some(url.clone()),
            _ => None,
        })
        .expect("resolve sink config");

        let sink = SinkClient::new(cfg.url.clone(), cfg.auth.clone()).expect("build sink client");
        Flusher::new(queue, sink, cfg)
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
        let retained = queue.read_all_tolerant().expect("read");
        assert_eq!(retained.len(), 1);
        assert_eq!(retained[0].content, "keep me");
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

        let retained = queue.read_all_tolerant().expect("read");
        assert_eq!(retained.len(), 1);
        assert_eq!(retained[0].content, "keep me");

        let _ = server.join();
    }
}

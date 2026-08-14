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
                        // The whole chain, not just the outermost context.
                        // `anyhow`'s plain `Display` prints only the context a
                        // caller added, so a refused connection reported itself
                        // as "POST to sink" — the attempt, never the cause. The
                        // reason reaches the log, and the log is all tarcie has
                        // to say that delivery has stopped.
                        failure = Some(format!("{e:#}"));
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
    use crate::test_sink::{spawn_silent_sink, spawn_sink, OK, UNREACHABLE};
    use tempfile::TempDir;
    use uuid::Uuid;

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

    fn config_for(url: &str, batch_max: Option<usize>) -> SinkConfig {
        let url = url.to_string();
        let batch = batch_max.map(|n| n.to_string());
        SinkConfig::resolve(|key| match key {
            "TARCIE_SINK_URL" => Some(url.clone()),
            "TARCIE_BATCH_MAX" => batch.clone(),
            _ => None,
        })
        .expect("resolve sink config")
    }

    fn flusher_for(url: &str, queue: Arc<JsonlQueue>) -> Flusher {
        flusher_with_batch(url, queue, None)
    }

    /// A flusher over the production sink client, bounded as a real run is.
    fn flusher_with_batch(url: &str, queue: Arc<JsonlQueue>, batch_max: Option<usize>) -> Flusher {
        let cfg = config_for(url, batch_max);
        let sink = SinkClient::new(cfg.url.clone(), cfg.auth.clone()).expect("build sink client");
        Flusher::new(queue, sink, cfg)
    }

    /// A flusher whose requests carry no bound at all.
    ///
    /// A paused clock advances to any deadline that exists the moment the
    /// runtime idles, and waiting on a real socket idles it. A test that needs
    /// a real exchange to succeed on a paused clock therefore has to run
    /// without the bound. `sink::client` proves the bound itself, on a real
    /// clock, where a silent sink and a healthy one can be told apart.
    fn flusher_unbounded(url: &str, queue: Arc<JsonlQueue>, batch_max: Option<usize>) -> Flusher {
        let cfg = config_for(url, batch_max);
        let sink = SinkClient::with_timeout(cfg.url.clone(), cfg.auth.clone(), None)
            .expect("build sink client");
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
    async fn a_deferral_names_the_cause_and_not_only_the_attempt() {
        // The reason is the whole account of why delivery stopped. It goes to
        // the log, and tarcie has no other way to say it. "POST to sink" names
        // what was tried, not what went wrong, so the reason carries the chain
        // under it.
        let (queue, _dir) = temp_queue();
        queue.append(&note("keep me"), 1000).expect("append");

        let flusher = flusher_for(UNREACHABLE, Arc::clone(&queue));

        match flusher.flush_with_retry().await.expect("flush") {
            FlushResult::Deferred { reason } => assert!(
                reason.to_lowercase().contains("connect"),
                "the reason names the connection failure, got: {reason}"
            ),
            FlushResult::Empty => panic!("expected deferral, got empty"),
            FlushResult::Success { .. } => panic!("expected deferral, got success"),
        }
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
    async fn a_sink_that_never_answers_is_given_up_on_rather_than_waited_out() {
        // What this holds is that the production path has a deadline at all.
        // On a paused clock the flush ends at the first deadline it has, so
        // this cannot tell a silent sink from a healthy one, and does not try
        // to — `sink::client` proves the bound discriminates, on a real clock.
        //
        // What it does prove is the defect that prompted it. With no bound in
        // `SinkClient::new` there is no deadline for the clock to advance to,
        // the flush never returns, and the guard below is what reports it. The
        // background flusher is a single task, so a flush that never returns
        // takes every later flush with it, for the rest of the session, and
        // says nothing while it does.
        let (queue, _dir) = temp_queue();
        queue.append(&note("keep me"), 1000).expect("append");

        let (url, _sink) = spawn_silent_sink();
        let flusher = flusher_for(&url, Arc::clone(&queue));

        // The guard is the assertion. Every wait inside the flush is bounded,
        // so on a paused clock the flush returns at a virtual deadline well
        // inside this one. Without a bound on the request there is no deadline
        // at all, and this reports that instead of hanging the suite.
        let result = tokio::time::timeout(Duration::from_secs(600), flusher.flush_with_retry())
            .await
            .expect("the flush gives up on a silent sink rather than waiting on it forever")
            .expect("flush");

        match result {
            FlushResult::Deferred { reason } => assert!(
                reason.to_lowercase().contains("timed out"),
                "the deferral names the timeout, got: {reason}"
            ),
            FlushResult::Empty => panic!("expected deferral, got empty"),
            FlushResult::Success { .. } => panic!("expected deferral, got success"),
        }

        // The reliability contract holds through a timeout as well.
        assert_eq!(still_owed(&queue), ["keep me"]);
    }

    #[tokio::test(start_paused = true)]
    async fn a_partial_multi_batch_delivery_retries_only_what_is_owed() {
        // The sink accepts the first batch of two, then the listener closes and
        // the rest of the flush fails. The accepted batch must not be offered
        // again — resending it would duplicate what the sink already holds.
        //
        // The first batch has to genuinely reach the sink, and the paused clock
        // is what keeps the retry backoff from costing fourteen real seconds.
        // The two cannot hold together with a bound on the request, so this
        // runs without one. See `flusher_unbounded`.
        let (queue, _dir) = temp_queue();
        for i in 0..4 {
            queue.append(&note(&format!("n{i}")), 1000).expect("append");
        }

        let (url, server) = spawn_sink(OK);
        let flusher = flusher_unbounded(&url, Arc::clone(&queue), Some(2));

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

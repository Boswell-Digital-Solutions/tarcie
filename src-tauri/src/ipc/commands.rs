use crate::constraints::*;
use crate::flusher::FlushResult;
use crate::model::{EventType, TarcieEvent};
use crate::state::AppState;
use regex::Regex;
use std::sync::Arc;
use std::time::Instant;
use tauri::State;
use uuid::Uuid;

/// Appended to content that was shortened, so truncation is visible rather
/// than silent.
const TRUNCATION_MARKER: &str = " […truncated]";

fn clamp_bytes(mut s: String, max: usize) -> String {
    if s.as_bytes().len() <= max {
        return s.trim().to_string();
    }

    // Reserve room for the marker, so the returned value honours `max` instead
    // of exceeding it by the marker's own length. A ceiling too small to hold
    // the marker spends the whole budget on content and goes without it.
    let marker_len = TRUNCATION_MARKER.as_bytes().len();
    let fits_marker = max >= marker_len;
    let budget = if fits_marker { max - marker_len } else { max };

    // `pop` removes whole characters, so the result stays valid UTF-8 even
    // when the budget lands inside a multi-byte character.
    while s.as_bytes().len() > budget {
        s.pop();
    }

    let mut out = s.trim().to_string();
    if fits_marker {
        out.push_str(TRUNCATION_MARKER);
    }
    out
}

/// What a tag looks like, in one place.
///
/// The whole run of tag characters is matched, not just the first
/// `MAX_TAG_CHARS` of it, so an overlong tag is consumed along with its
/// overflow instead of leaving the remainder behind in the content.
fn tag_re() -> &'static Regex {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| Regex::new(r"#([a-zA-Z0-9_-]+)").unwrap())
}

/// Whether the text says anything that is not a tag.
///
/// Every tag is taken out to decide this, not only the first. `extract_tag`
/// takes the first as the context and leaves the rest in the content by
/// design, but a note made only of tags carries no observation to place under
/// them.
fn has_text_of_its_own(content: &str) -> bool {
    !tag_re().replace_all(content, "").trim().is_empty()
}

fn extract_tag(content: &str) -> (String, String) {
    let re = tag_re();

    if let Some(m) = re.find(content) {
        let matched = &content[m.start() + 1..m.end()];
        let tag: String = matched.chars().take(MAX_TAG_CHARS).collect();
        let cleaned = content.replacen(m.as_str(), "", 1).trim().to_string();
        (tag, cleaned)
    } else {
        (DEFAULT_CONTEXT.to_string(), content.trim().to_string())
    }
}

fn now_mono_ms(start: Instant) -> u64 {
    start.elapsed().as_millis() as u64
}

fn build_event(
    device_id: Uuid,
    mono_start: Instant,
    event_type: EventType,
    content: String,
    app_context: String,
) -> TarcieEvent {
    TarcieEvent {
        id: Uuid::new_v4(),
        device_id,
        timestamp_utc: chrono::Utc::now(),
        timestamp_mono_ms: now_mono_ms(mono_start),
        event_type,
        content,
        app_context,
        source_version: SOURCE_VERSION.to_string(),
    }
}

/// The capture path behind `capture_note`, over a plain `&AppState`.
///
/// The command adds only Tauri's state extraction, so this function holds the
/// whole path — clamp, tag, build, append — and a test can drive it without a
/// running application.
pub fn capture_note_into(state: &AppState, content: String) -> Result<(), String> {
    let content = clamp_bytes(content, MAX_CONTENT_BYTES);

    // A note must say something of its own. This turns away an empty box,
    // whitespace, a tag alone, and a string of tags alike. A tag names the
    // context an observation belongs to; with no observation there is nothing
    // to place under it.
    //
    // This is the one place that decides what counts as a note. The overlay
    // stops an obviously empty box before it gets here and stops at that, so
    // the tag pattern lives in one place rather than in two that can drift.
    //
    // The queue is what the check protects. An event with no text is durable,
    // delivered, and archived for good, like any other.
    if !has_text_of_its_own(&content) {
        return Err("a note needs text of its own".to_string());
    }

    let (tag, cleaned) = extract_tag(&content);

    let ctx = clamp_bytes(tag, MAX_CONTEXT_CHARS);
    let cleaned = clamp_bytes(cleaned, MAX_CONTENT_BYTES);

    let ev = build_event(
        state.device_id,
        state.mono_start,
        EventType::Note,
        cleaned,
        ctx,
    );

    state
        .queue
        .append(&ev, state.cfg.queue_max_events)
        .map_err(|e| e.to_string())
}

/// The capture path behind `capture_marker`, over a plain `&AppState`.
pub fn capture_marker_into(state: &AppState, reason: Option<String>) -> Result<(), String> {
    let reason = reason.map(|r| clamp_bytes(r, MAX_CONTENT_BYTES));
    let ev = build_event(
        state.device_id,
        state.mono_start,
        EventType::Marker { reason },
        String::new(),
        DEFAULT_CONTEXT.to_string(),
    );

    state
        .queue
        .append(&ev, state.cfg.queue_max_events)
        .map_err(|e| e.to_string())
}

/// The flush path behind `flush_now`, over a plain `&AppState`.
///
/// The reply is the string the caller receives, so the mapping from
/// `FlushResult` to that string is what a test can hold.
pub async fn flush_now_on(state: &AppState) -> Result<String, String> {
    match state.flusher.flush_with_retry().await {
        Ok(FlushResult::Empty) => Ok("empty".into()),
        Ok(FlushResult::Success { count }) => Ok(format!("ok:{}", count)),
        Ok(FlushResult::Deferred { reason }) => Ok(format!("deferred:{}", reason)),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn capture_note(content: String, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    capture_note_into(state.inner(), content)
}

#[tauri::command]
pub async fn capture_marker(
    reason: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    capture_marker_into(state.inner(), reason)
}

#[tauri::command]
pub async fn flush_now(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    flush_now_on(state.inner()).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flusher::Flusher;
    use crate::queue::jsonl::JsonlQueue;
    use crate::sink::client::SinkClient;
    use crate::sink::config::SinkConfig;
    use crate::test_sink::{spawn_sink, OK, UNREACHABLE};
    use tempfile::TempDir;

    /// An application state whose queue is isolated in a temporary directory
    /// and whose sink is the given URL.
    ///
    /// The `TempDir` comes back with it: dropping it removes the queue, so it
    /// has to outlive the state.
    fn state_for(url: &str) -> (Arc<AppState>, TempDir) {
        let dir = TempDir::new().expect("create temp dir");
        let url = url.to_string();

        let cfg = SinkConfig::resolve(|key| match key {
            "TARCIE_SINK_URL" => Some(url.clone()),
            _ => None,
        })
        .expect("resolve sink config");

        let queue = Arc::new(
            JsonlQueue::new_in(dir.path().join("queue"), dir.path().join("queue/sent"))
                .expect("build the queue"),
        );
        let sink =
            SinkClient::new(cfg.url.clone(), cfg.auth.clone()).expect("build the sink client");
        let flusher = Arc::new(Flusher::new(Arc::clone(&queue), sink, cfg.clone()));

        let state = Arc::new(AppState {
            cfg,
            queue,
            flusher,
            device_id: Uuid::new_v4(),
            mono_start: Instant::now(),
        });

        (state, dir)
    }

    /// What the live queue holds, without claiming it.
    fn queued(state: &AppState) -> Vec<TarcieEvent> {
        state.queue.read_all_tolerant().expect("read the queue")
    }

    // --- 4. Content clamping ---------------------------------------------

    #[test]
    fn content_within_the_limit_is_only_trimmed() {
        assert_eq!(clamp_bytes("  hello  ".to_string(), 100), "hello");
    }

    #[test]
    fn content_exactly_at_the_limit_is_left_alone() {
        let exact = "a".repeat(100);
        assert_eq!(clamp_bytes(exact.clone(), 100), exact);
    }

    #[test]
    fn oversized_content_is_clamped_rather_than_rejected() {
        let huge = "a".repeat(MAX_CONTENT_BYTES + 500);
        let clamped = clamp_bytes(huge, MAX_CONTENT_BYTES);

        assert!(clamped.starts_with("aaa"), "the leading content is kept");
        assert!(
            clamped.ends_with(TRUNCATION_MARKER),
            "truncation is disclosed, not silent"
        );
        assert!(clamped.len() < MAX_CONTENT_BYTES + 500, "the string actually shrank");
    }

    #[test]
    fn clamping_never_splits_a_multibyte_character() {
        // Each '€' is three bytes, so the budget lands mid-character. Popping
        // whole characters keeps the result valid UTF-8 and inside the limit.
        let clamped = clamp_bytes("€".repeat(10), 20);

        assert!(clamped.starts_with('€'));
        assert!(clamped.as_bytes().len() <= 20);
        assert!(clamped.ends_with(TRUNCATION_MARKER));
        // 20 bytes less the 15-byte marker leaves room for one whole '€'.
        assert_eq!(clamped.matches('€').count(), 1);
    }

    #[test]
    fn a_clamped_result_stays_within_its_limit() {
        // The marker is budgeted for, not appended on top of a full-width
        // string, so the ceiling holds.
        let clamped = clamp_bytes("b".repeat(200), 100);
        assert_eq!(clamped.as_bytes().len(), 100);
        assert!(clamped.ends_with(TRUNCATION_MARKER));
    }

    #[test]
    fn a_limit_too_small_for_the_marker_truncates_without_it() {
        let clamped = clamp_bytes("b".repeat(200), 4);
        assert!(clamped.as_bytes().len() <= 4);
        assert_eq!(clamped, "bbbb");
    }

    #[test]
    fn every_clamped_result_honours_its_limit() {
        // The ceiling holds across the boundary around the marker's own width.
        for max in [0, 1, 14, 15, 16, 50, MAX_CONTENT_BYTES] {
            let clamped = clamp_bytes("c".repeat(MAX_CONTENT_BYTES + 64), max);
            assert!(
                clamped.as_bytes().len() <= max,
                "max {max} produced {} bytes",
                clamped.as_bytes().len()
            );
        }
    }

    // --- 5. Tag extraction ------------------------------------------------

    #[test]
    fn a_leading_tag_becomes_the_context_and_leaves_the_content() {
        let (tag, content) = extract_tag("#meeting discuss roadmap");
        assert_eq!(tag, "meeting");
        assert_eq!(content, "discuss roadmap");
    }

    #[test]
    fn content_without_a_tag_falls_back_to_the_default_context() {
        let (tag, content) = extract_tag("  just a note  ");
        assert_eq!(tag, DEFAULT_CONTEXT);
        assert_eq!(content, "just a note");
    }

    #[test]
    fn a_tag_is_found_anywhere_in_the_content() {
        let (tag, content) = extract_tag("ship the #release today");
        assert_eq!(tag, "release");
        // Removing the tag in place leaves the surrounding spacing behind.
        assert_eq!(content, "ship the  today");
    }

    #[test]
    fn only_the_first_tag_is_extracted() {
        let (tag, content) = extract_tag("#one and #two");
        assert_eq!(tag, "one");
        assert_eq!(content, "and #two", "later tags stay in the captured text");
    }

    #[test]
    fn a_tag_stops_at_the_first_disallowed_character() {
        let (tag, content) = extract_tag("#build.fast go");
        assert_eq!(tag, "build");
        assert_eq!(content, ".fast go");
    }

    #[test]
    fn a_bare_hash_is_not_a_tag() {
        let (tag, content) = extract_tag("# not a tag");
        assert_eq!(tag, DEFAULT_CONTEXT);
        assert_eq!(content, "# not a tag");
    }

    #[test]
    fn an_overlong_tag_is_clamped_and_fully_consumed() {
        let overlong = "a".repeat(MAX_TAG_CHARS + 5);
        let (tag, content) = extract_tag(&format!("#{overlong} rest"));

        assert_eq!(tag.len(), MAX_TAG_CHARS, "the tag is clamped");
        assert_eq!(
            content, "rest",
            "the overflow leaves with the tag rather than staying in the content"
        );
    }

    #[test]
    fn tag_extraction_accepts_digits_underscores_and_hyphens() {
        for (input, expected) in [
            ("#v1_2-3 note", "v1_2-3"),
            ("#UPPER note", "UPPER"),
            ("#123 note", "123"),
        ] {
            let (tag, _) = extract_tag(input);
            assert_eq!(tag, expected, "input: {input}");
        }
    }

    // --- 8. The command layer ---------------------------------------------
    //
    // The tests above hold the parts. These hold the whole path a capture
    // takes: what the user typed goes in, and what the queue keeps comes out.

    #[test]
    fn a_captured_note_reaches_the_queue_with_its_tag_as_the_context() {
        let (state, _dir) = state_for(UNREACHABLE);

        capture_note_into(&state, "#meeting discuss roadmap".to_string()).expect("capture");

        let events = queued(&state);
        assert_eq!(events.len(), 1, "the capture was queued");
        assert_eq!(events[0].content, "discuss roadmap");
        assert_eq!(events[0].app_context, "meeting");
        assert!(matches!(events[0].event_type, EventType::Note));
    }

    #[test]
    fn a_note_without_a_tag_lands_under_the_default_context() {
        let (state, _dir) = state_for(UNREACHABLE);

        capture_note_into(&state, "  just a thought  ".to_string()).expect("capture");

        let events = queued(&state);
        assert_eq!(events[0].content, "just a thought");
        assert_eq!(events[0].app_context, DEFAULT_CONTEXT);
    }

    #[test]
    fn a_captured_note_carries_the_device_identity_and_the_source_version() {
        let (state, _dir) = state_for(UNREACHABLE);

        capture_note_into(&state, "a note".to_string()).expect("capture");

        let events = queued(&state);
        assert_eq!(
            events[0].device_id, state.device_id,
            "the sink can tell which machine captured it"
        );
        assert_eq!(events[0].source_version, SOURCE_VERSION);
    }

    #[test]
    fn an_oversized_note_is_queued_clamped_rather_than_refused() {
        // Losing the capture would cost more than losing its tail.
        let (state, _dir) = state_for(UNREACHABLE);

        capture_note_into(&state, "a".repeat(MAX_CONTENT_BYTES * 2)).expect("capture");

        let events = queued(&state);
        assert_eq!(events.len(), 1, "the capture is kept, not rejected");
        assert!(events[0].content.as_bytes().len() <= MAX_CONTENT_BYTES);
        assert!(
            events[0].content.ends_with(TRUNCATION_MARKER),
            "the queued event discloses that it was shortened"
        );
    }

    #[test]
    fn a_note_with_no_text_of_its_own_never_reaches_the_queue() {
        // An event with no text is as durable as any other: queued, fsynced,
        // delivered, and archived for good. The cheapest place to stop one is
        // before it is written.
        //
        // A tag on its own is refused with the rest. The tag names a context
        // for an observation; on its own there is no observation to place.
        let (state, _dir) = state_for(UNREACHABLE);

        for nothing in ["", "   ", "\t\n  ", "#bug", "  #bug  ", "#a #b"] {
            assert!(
                capture_note_into(&state, nothing.to_string()).is_err(),
                "{nothing:?} carries no text of its own"
            );
        }

        assert!(queued(&state).is_empty(), "nothing was written");
    }

    #[test]
    fn a_tag_with_text_beside_it_is_still_a_note() {
        // The guard turns away a tag with nothing behind it, never a tagged
        // observation.
        let (state, _dir) = state_for(UNREACHABLE);

        capture_note_into(&state, "#bug the overlay froze".to_string()).expect("capture");

        let events = queued(&state);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].content, "the overlay froze");
        assert_eq!(events[0].app_context, "bug");
    }

    #[test]
    fn a_bare_hash_is_text_and_is_kept() {
        // "#" alone matches no tag, so it is content like any other character.
        let (state, _dir) = state_for(UNREACHABLE);

        capture_note_into(&state, "# not a tag".to_string()).expect("capture");

        assert_eq!(queued(&state)[0].content, "# not a tag");
    }

    #[test]
    fn a_captured_marker_has_no_content_and_the_default_context() {
        let (state, _dir) = state_for(UNREACHABLE);

        capture_marker_into(&state, None).expect("capture");

        let events = queued(&state);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].content, "");
        assert_eq!(events[0].app_context, DEFAULT_CONTEXT);
        assert!(matches!(
            events[0].event_type,
            EventType::Marker { reason: None }
        ));
    }

    #[test]
    fn a_marker_reason_is_carried_through_to_the_queue() {
        let (state, _dir) = state_for(UNREACHABLE);

        capture_marker_into(&state, Some("  stepping away  ".to_string())).expect("capture");

        match &queued(&state)[0].event_type {
            EventType::Marker { reason } => assert_eq!(reason.as_deref(), Some("stepping away")),
            EventType::Note => panic!("expected a marker, got a note"),
        }
    }

    #[test]
    fn captures_keep_their_order_in_the_queue() {
        let (state, _dir) = state_for(UNREACHABLE);

        for i in 0..3 {
            capture_note_into(&state, format!("n{i}")).expect("capture");
        }

        let contents: Vec<String> = queued(&state).iter().map(|e| e.content.clone()).collect();
        assert_eq!(contents, ["n0", "n1", "n2"]);
    }

    #[test]
    fn every_capture_gets_its_own_event_id() {
        // The sink deduplicates on `id`, so two captures sharing one would
        // silently collapse into a single event downstream.
        let (state, _dir) = state_for(UNREACHABLE);

        capture_note_into(&state, "first".to_string()).expect("capture");
        capture_note_into(&state, "second".to_string()).expect("capture");
        capture_marker_into(&state, None).expect("capture");

        let events = queued(&state);
        let mut ids: Vec<_> = events.iter().map(|e| e.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 3, "three captures, three ids");
    }

    #[tokio::test]
    async fn flush_now_reports_empty_when_there_is_nothing_to_send() {
        let (state, _dir) = state_for(UNREACHABLE);

        // The sink is unreachable; contacting it at all would fail the test.
        assert_eq!(flush_now_on(&state).await.expect("flush"), "empty");
    }

    #[tokio::test]
    async fn flush_now_reports_the_count_it_delivered() {
        let (url, server) = spawn_sink(OK);
        let (state, _dir) = state_for(&url);

        capture_note_into(&state, "one".to_string()).expect("capture");
        capture_note_into(&state, "two".to_string()).expect("capture");

        assert_eq!(flush_now_on(&state).await.expect("flush"), "ok:2");

        let _ = server.join();
    }

    #[tokio::test(start_paused = true)]
    async fn flush_now_reports_a_deferral_and_keeps_the_capture() {
        let (state, _dir) = state_for(UNREACHABLE);
        capture_note_into(&state, "keep me".to_string()).expect("capture");

        let reply = flush_now_on(&state).await.expect("flush");
        assert!(
            reply.starts_with("deferred:"),
            "an unreachable sink defers, got {reply}"
        );

        // The reliability contract: a deferral never costs a capture.
        let owed = state.queue.claim().expect("claim");
        assert_eq!(owed.events()[0].content, "keep me");
    }
}

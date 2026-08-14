use crate::constraints::*;
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

fn extract_tag(content: &str) -> (String, String) {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    // The whole run of tag characters is matched, not just the first
    // MAX_TAG_CHARS of it, so an overlong tag is consumed along with its
    // overflow instead of leaving the remainder behind in the content.
    let re = RE.get_or_init(|| Regex::new(r"#([a-zA-Z0-9_-]+)").unwrap());

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

#[tauri::command]
pub async fn capture_note(content: String, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let mono_start = state.mono_start;
    let device_id = state.device_id;

    let content = clamp_bytes(content, MAX_CONTENT_BYTES);
    let (tag, cleaned) = extract_tag(&content);

    let ctx = clamp_bytes(tag, MAX_CONTEXT_CHARS);
    let cleaned = clamp_bytes(cleaned, MAX_CONTENT_BYTES);

    let ev = build_event(device_id, mono_start, EventType::Note, cleaned, ctx);

    state
        .queue
        .append(&ev, state.cfg.queue_max_events)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn capture_marker(reason: Option<String>, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let mono_start = state.mono_start;
    let device_id = state.device_id;

    let reason = reason.map(|r| clamp_bytes(r, MAX_CONTENT_BYTES));
    let ev = build_event(
        device_id,
        mono_start,
        EventType::Marker { reason },
        String::new(),
        DEFAULT_CONTEXT.to_string(),
    );

    state
        .queue
        .append(&ev, state.cfg.queue_max_events)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn flush_now(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    match state.flusher.flush_with_retry().await {
        Ok(crate::flusher::FlushResult::Empty) => Ok("empty".into()),
        Ok(crate::flusher::FlushResult::Success { count }) => Ok(format!("ok:{}", count)),
        Ok(crate::flusher::FlushResult::Deferred { reason }) => Ok(format!("deferred:{}", reason)),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}

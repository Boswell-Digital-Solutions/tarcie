// TARCIE v1 CONSTRAINTS - DO NOT REMOVE
// 1. If capture takes > 5s, REVERT.
// 2. Write-only from UI perspective. No readback surfaces in v1.
// 3. No categorization logic. SMITH does grouping.
// 4. No AI / LLMs. Raw strings only.
// 5. UI must be small and non-blocking.

pub const SOURCE_VERSION: &str = "tarcie-v1.0.0";
pub const DEFAULT_CONTEXT: &str = "General";

pub const MAX_CONTEXT_CHARS: usize = 64;
pub const MAX_TAG_CHARS: usize = 32;
pub const MAX_CONTENT_BYTES: usize = 10 * 1024; // 10KB

pub const DEFAULT_FLUSH_INTERVAL_SECS: u64 = 300;

// tokio::time::interval panics on a zero period. That panic lands in the
// spawned flush task, where nothing reports it, so the background flusher
// would be gone for the rest of the session without a word.
pub const MIN_FLUSH_INTERVAL_SECS: u64 = 1;
pub const DEFAULT_BATCH_MAX: usize = 200;
pub const DEFAULT_QUEUE_MAX_EVENTS: usize = 10_000;

pub const HOTKEY: &str = "Ctrl+Alt+T";
pub const HOTKEY_DEBOUNCE_MS: u64 = 500;

// How long a close waits for the final flush before the window goes. Every
// capture is already durable on disk, so a slow sink delays the shutdown by
// this much at most and never costs an event.
pub const SHUTDOWN_FLUSH_SECS: u64 = 5;

// How long one POST to the sink may take before it is given up on. reqwest
// applies no bound of its own, so a sink that accepts the connection and then
// says nothing holds the request open with nothing to report. The background
// flusher is one task, so that single flush would end delivery for the rest of
// the session, and the log would carry no word of it.
//
// Four attempts and the backoff between them stay inside the default flush
// interval: 4 x 30s of waiting, plus 2s + 4s + 8s, is 134s against 300s.
pub const SINK_REQUEST_TIMEOUT_SECS: u64 = 30;

// How long a delivered batch stays in the archive, and how much of it is kept.
//
// The archive held every capture ever made, for the life of the installation.
// Nothing in tarcie reads it, so its value is forensic and manual — worth
// keeping, not worth keeping forever.
//
// Sized for an ordinary desktop. A typical note is about 310 bytes on the line,
// so a hundred captures a day costs roughly 11 MB a year and never approaches
// the ceiling. The ceiling is there for content that runs to MAX_CONTENT_BYTES,
// where the same hundred a day would reach about 370 MB a year.
//
// The two work together. The period decides what is old enough to drop; the
// ceiling bounds the worst case whatever the period says.
pub const SENT_RETENTION_DAYS: i64 = 90;
pub const SENT_MAX_BYTES: u64 = 256 * 1024 * 1024;

// The log is bounded. One previous file is kept, so the pair costs at most
// twice this. A log that grows until the disk is gone would take the queue
// with it, and the queue is the reliability contract.
pub const MAX_LOG_BYTES: u64 = 1024 * 1024;

// A deferral carries the sink's response text, which is the sink's to decide
// the length of. One line never gets to be the reason the log fills.
pub const MAX_LOG_LINE_CHARS: usize = 2048;

// MONOTONIC CLOCK LIMITATION:
// timestamp_mono_ms resets on app restart. Use only for relative timing
// within a session. Cross-session ordering relies on timestamp_utc.

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

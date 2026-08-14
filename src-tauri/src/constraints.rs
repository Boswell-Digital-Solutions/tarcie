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
pub const DEFAULT_BATCH_MAX: usize = 200;
pub const DEFAULT_QUEUE_MAX_EVENTS: usize = 10_000;

pub const HOTKEY: &str = "Ctrl+Alt+T";
pub const HOTKEY_DEBOUNCE_MS: u64 = 500;

// How long a close waits for the final flush before the window goes. Every
// capture is already durable on disk, so a slow sink delays the shutdown by
// this much at most and never costs an event.
pub const SHUTDOWN_FLUSH_SECS: u64 = 5;

// MONOTONIC CLOCK LIMITATION:
// timestamp_mono_ms resets on app restart. Use only for relative timing
// within a session. Cross-session ordering relies on timestamp_utc.

# 1. Overview

Tarcie is a friction-free capture tool for notes and markers. A global hotkey (`Ctrl+Alt+T`) pops a 480x140px overlay window. The user types a note or drops a timestamp marker. The event is appended to a JSONL file queue with fsync durability, and a background flusher periodically batches events to an HTTP sink endpoint.

## Design Philosophy

Tarcie is strictly **write-only** in v1. There is no readback surface, no categorization logic, and no AI processing. Raw strings go in, raw strings go out. SMITH handles grouping and analysis downstream.

## At a Glance

| Metric | Value |
|--------|-------|
| Rust LOC | 636 |
| IPC Commands | 3 |
| Frontend | Vanilla TypeScript (main.ts + styles.css + index.html) |
| Framework | Tauri 2.0 (no SvelteKit, no UI framework) |
| Edition | Rust 2024 |
| Version | 1.0.0 |

## What Tarcie Does

1. Listens for `Ctrl+Alt+T` global hotkey
2. Shows/hides a minimal overlay window
3. Accepts text notes (with optional `#tag`) or timestamp markers
4. Appends events to a local JSONL queue (fsync-durable)
5. Flushes queued events to an HTTP sink in batches on a timer
6. Attempts a graceful flush on shutdown (5-second timeout)

## What Tarcie Does Not Do

- No readback or browsing of captured events
- No categorization, tagging intelligence, or grouping
- No AI/LLM processing of any kind
- No complex UI beyond the capture overlay

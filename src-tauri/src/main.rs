// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod constraints;
mod flusher;
mod ipc;
mod model;
mod queue;
mod schedule;
mod sink;
mod state;
#[cfg(test)]
mod test_sink;
mod util;

use crate::constraints::*;
use crate::flusher::FlushResult;
use crate::queue::jsonl::JsonlQueue;
use crate::sink::client::SinkClient;
use crate::sink::config::SinkConfig;
use crate::state::AppState;
use crate::util::device::load_or_create_device_id;
use crate::util::log;
use crate::util::paths::logs_dir;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{Manager, WebviewWindow};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

/// The capture hotkey, built from the string the documentation states.
///
/// The binding used to be assembled from `Modifiers` and `Code` beside a
/// `HOTKEY` constant that nothing read. The two could drift apart, and the
/// documented hotkey would then name a key combination that does nothing.
fn capture_shortcut() -> anyhow::Result<Shortcut> {
    HOTKEY
        .parse::<Shortcut>()
        .map_err(|e| anyhow::anyhow!("parse the capture hotkey {HOTKEY:?}: {e}"))
}

/// Say what a flush did, and hand the outcome back to the caller.
///
/// A deferral is the queue keeping its promise, not a fault. It is also the
/// only word anyone gets that captures are not arriving: tarcie has no
/// readback surface, so an unreported deferral leaves a sink that has been
/// refusing for days looking like a sink with nothing to do.
fn report(result: anyhow::Result<FlushResult>) -> Option<FlushResult> {
    match result {
        Ok(FlushResult::Deferred { reason }) => {
            log::write(format_args!("flush deferred, every event kept: {}", reason));
            Some(FlushResult::Deferred { reason })
        }
        Ok(other) => Some(other),
        Err(e) => {
            log::write(format_args!("background flush error: {}", e));
            None
        }
    }
}

fn toggle_window(window: &WebviewWindow) {
    let visible = window.is_visible().unwrap_or(false);
    if visible {
        let _ = window.hide();
    } else {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            // The log opens first, so everything after it can report. A log
            // that will not open is not a reason to refuse to capture: the
            // reports fall back to stderr and the application carries on.
            match logs_dir() {
                Ok(dir) => {
                    if let Err(e) = log::init_in(&dir) {
                        eprintln!("tarcie: could not open the log: {e}");
                    }
                }
                Err(e) => eprintln!("tarcie: could not resolve the log dir: {e}"),
            }

            let cfg = SinkConfig::from_env()?;

            // Where events go and how often. The URL is reported without any
            // credentials it carries, and the auth token is never reported.
            match cfg.flush_at {
                Some(at) => log::write(format_args!(
                    "started, sink {}, delivering daily at {}, checked every {}s",
                    cfg.url_without_credentials(),
                    at.format("%H:%M"),
                    cfg.flush_interval_secs
                )),
                None => log::write(format_args!(
                    "started, sink {}, flush every {}s",
                    cfg.url_without_credentials(),
                    cfg.flush_interval_secs
                )),
            }

            let queue = Arc::new(JsonlQueue::new()?);

            // The archive is bounded whenever a batch is added to it. A run
            // that adds nothing would otherwise never revisit what is already
            // there, so the retention period is kept here as well.
            queue.bound_archive();

            let sink = SinkClient::new(cfg.url.clone(), cfg.auth.clone())?;
            let flusher = Arc::new(flusher::Flusher::new(
                Arc::clone(&queue),
                sink,
                cfg.clone(),
            ));

            let device_id = load_or_create_device_id()?;
            let mono_start = Instant::now();

            let state = Arc::new(AppState {
                cfg: cfg.clone(),
                queue: Arc::clone(&queue),
                flusher: Arc::clone(&flusher),
                device_id,
                mono_start,
            });

            app.manage(state);

            let window = app.get_webview_window("main").expect("main window");
            let last_toggle = Arc::new(Mutex::new(Instant::now() - Duration::from_millis(HOTKEY_DEBOUNCE_MS)));

            // Register the global shortcut the documentation states.
            let shortcut = capture_shortcut()?;

            {
                let window = window.clone();
                let last_toggle = last_toggle.clone();

                app.global_shortcut().on_shortcut(shortcut, move |_app, _shortcut, _event| {
                    let mut last = last_toggle.lock().unwrap();
                    if last.elapsed() < Duration::from_millis(HOTKEY_DEBOUNCE_MS) {
                        return;
                    }
                    *last = Instant::now();
                    toggle_window(&window);
                })?;
            }

            // Background flush loop. The ticker is the same either way; what
            // differs is whether every tick delivers, or only the first tick
            // after today's target time has passed.
            let flusher_bg = Arc::clone(&flusher);
            let interval = Duration::from_secs(cfg.flush_interval_secs);
            let flush_at = cfg.flush_at;
            let marker = util::paths::schedule_marker_path().ok();

            tauri::async_runtime::spawn(async move {
                let mut ticker = tokio::time::interval(interval);
                loop {
                    ticker.tick().await;

                    let Some(target) = flush_at else {
                        report(flusher_bg.flush_with_retry().await);
                        continue;
                    };

                    let now = chrono::Local::now().naive_local();
                    let last = marker.as_deref().and_then(schedule::last_delivery);

                    if !schedule::is_due(now, target, last) {
                        continue;
                    }

                    // A claim is bounded, so a day that captured more than one
                    // claim holds needs more than one round. On an interval
                    // the next cycle is minutes away; here it is a day, and a
                    // backlog would never catch up.
                    let mut drained = false;
                    for _ in 0..MAX_SCHEDULED_ROUNDS {
                        match report(flusher_bg.flush_with_retry().await) {
                            Some(FlushResult::Empty) => {
                                drained = true;
                                break;
                            }
                            Some(FlushResult::Success { .. }) => continue,
                            _ => break,
                        }
                    }

                    // The day is recorded only once the queue is clear. A
                    // deferral leaves it unrecorded, so the next tick tries
                    // again rather than waiting out the night on a sink that
                    // was briefly unreachable.
                    if drained {
                        if let Some(path) = marker.as_deref() {
                            if let Err(e) = schedule::record_delivery(path, now.date()) {
                                log::write(format_args!(
                                    "could not record the scheduled delivery: {e:#}"
                                ));
                            }
                        }
                    }
                }
            });

            // Graceful shutdown handler
            let flusher_close = Arc::clone(&flusher);
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { .. } = event {
                    let flusher = Arc::clone(&flusher_close);

                    // A window event arrives on the event loop thread, which
                    // has no Tokio runtime entered. `Handle::current()` panics
                    // there, so the final flush never ran. Tauri's own runtime
                    // is reachable from any thread and blocks on it safely.
                    let _ = tauri::async_runtime::block_on(async {
                        tokio::time::timeout(
                            Duration::from_secs(SHUTDOWN_FLUSH_SECS),
                            flusher.flush_with_retry(),
                        )
                        .await
                    });
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ipc::commands::capture_note,
            ipc::commands::capture_marker,
            ipc::commands::flush_now,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    // Only the test names the combination directly. The application builds it
    // from HOTKEY, which is the point.
    use tauri_plugin_global_shortcut::{Code, Modifiers};

    #[test]
    fn the_documented_hotkey_is_the_one_that_gets_registered() {
        // A hotkey that does not parse would only show up at launch, on the
        // user's machine, as a capture tool with no way in.
        assert_eq!(
            capture_shortcut().expect("parse the documented hotkey"),
            Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyT),
            "HOTKEY states Ctrl+Alt+T, so that is what gets registered"
        );
    }
}

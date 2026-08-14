// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod constraints;
mod flusher;
mod ipc;
mod model;
mod queue;
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
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{Manager, WebviewWindow};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};

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
            let cfg = SinkConfig::from_env()?;

            let queue = Arc::new(JsonlQueue::new()?);

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

            // Background flush loop
            let flusher_bg = Arc::clone(&flusher);
            let interval = Duration::from_secs(cfg.flush_interval_secs);
            tauri::async_runtime::spawn(async move {
                let mut ticker = tokio::time::interval(interval);
                loop {
                    ticker.tick().await;
                    match flusher_bg.flush_with_retry().await {
                        // A deferral is the queue keeping its promise, not a
                        // fault. It is also the only word anyone gets that
                        // captures are not arriving: tarcie has no readback
                        // surface, so an unreported deferral leaves a sink
                        // that has been refusing for days looking like a sink
                        // with nothing to do.
                        Ok(FlushResult::Deferred { reason }) => {
                            eprintln!("tarcie: flush deferred, every event kept: {}", reason);
                        }
                        Ok(FlushResult::Empty) | Ok(FlushResult::Success { .. }) => {}
                        Err(e) => eprintln!("tarcie: background flush error: {}", e),
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

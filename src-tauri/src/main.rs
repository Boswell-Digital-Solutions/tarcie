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
use crate::queue::jsonl::JsonlQueue;
use crate::sink::client::SinkClient;
use crate::sink::config::SinkConfig;
use crate::state::AppState;
use crate::util::device::load_or_create_device_id;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{Manager, WebviewWindow};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};

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

            // Register global shortcut Ctrl+Alt+T
            let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyT);

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
                    if let Err(e) = flusher_bg.flush_with_retry().await {
                        eprintln!("tarcie: background flush error: {}", e);
                    }
                }
            });

            // Graceful shutdown handler
            let flusher_close = Arc::clone(&flusher);
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { .. } = event {
                    let rt = tokio::runtime::Handle::current();
                    let flusher = Arc::clone(&flusher_close);
                    let _ = rt.block_on(async {
                        tokio::time::timeout(
                            Duration::from_secs(5),
                            flusher.flush_with_retry()
                        ).await
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

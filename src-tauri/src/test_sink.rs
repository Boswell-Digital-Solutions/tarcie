//! A one-shot HTTP sink for tests.
//!
//! The flusher and the command layer both need a sink they can drive. They
//! share this stub so both see the same behaviour.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Loopback port 1 is privileged and unbound, so a connection to it is
/// refused immediately rather than hanging.
pub const UNREACHABLE: &str = "http://127.0.0.1:1/ingest";

/// How long a silent sink stays silent. A test drives it on a paused clock and
/// finishes in a fraction of this; the process then exits and takes the
/// sleeping thread with it.
const SILENT_HOLD_SECS: u64 = 10;

/// The response of a sink that accepts the batch.
pub const OK: &str = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n";

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Consume one whole HTTP request, headers and declared body both, so the
/// client sees a complete exchange instead of a reset connection.
fn read_request(stream: &mut TcpStream) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 2048];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
        if let Some(head_end) = find(&buf, b"\r\n\r\n") {
            let head = String::from_utf8_lossy(&buf[..head_end]).to_lowercase();
            let body_len = head
                .lines()
                .find_map(|line| line.strip_prefix("content-length:"))
                .and_then(|v| v.trim().parse::<usize>().ok())
                .unwrap_or(0);
            if buf.len() >= head_end + 4 + body_len {
                break;
            }
        }
    }
    buf
}

/// A sink that accepts the connection, takes the whole request, and never
/// answers.
///
/// This is the failure an unreachable port cannot stand in for. A refused
/// connection fails at once and reports itself; a sink that goes quiet holds a
/// healthy connection open and reports nothing, so only a deadline ends the
/// wait.
///
/// The listener stays bound for as long as the thread holds it, so the retries
/// that follow reach the accept queue and go unanswered in the same way.
pub fn spawn_silent_sink() -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let addr = listener.local_addr().expect("local addr");

    let handle = thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let _ = read_request(&mut stream);
            thread::sleep(Duration::from_secs(SILENT_HOLD_SECS));
        }
    });

    (format!("http://{addr}/ingest"), handle)
}

/// A one-shot HTTP endpoint on an ephemeral port. The handle joins to the
/// request body it received, so a test can assert on what was sent.
pub fn spawn_sink(response: &'static str) -> (String, JoinHandle<Vec<u8>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let addr = listener.local_addr().expect("local addr");

    let handle = thread::spawn(move || {
        let mut received = Vec::new();
        if let Ok((mut stream, _)) = listener.accept() {
            received = read_request(&mut stream);
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
        received
    });

    (format!("http://{addr}/ingest"), handle)
}

// SPDX-License-Identifier: Apache-2.0
//
// Tiny self-contained HTTP server that serves the aphrody-terminal demo
// HTML on `127.0.0.1:18800`. Pure `std::net` — no external deps, no
// async runtime, no framework. The HTML is embedded at compile time via
// `include_str!` so the binary is fully self-contained.
//
// Run:   cargo run -p aphrody-wasm --example serve
// Stop:  Ctrl-C
//
// Why port 18800?
//   Port 18799 is reserved by `aphrody term` (CLAUDE.md §0.4) for the
//   live LLM-first terminal WS server. We pick 18800 so both can run
//   simultaneously without conflict.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Demo page embedded at build time. Sibling assets (none today) would be
/// embedded via additional `include_str!` / `include_bytes!` blocks.
const DEMO_HTML: &str = include_str!("aphrody-terminal-demo.html");

const BIND_ADDR: &str = "127.0.0.1:18800";

fn main() {
    let running = Arc::new(AtomicBool::new(true));
    let running_for_handler = Arc::clone(&running);

    // Ctrl-C handler: pure std, no `signal-hook` dep. On Windows + Unix the
    // SIGINT is delivered to the listener accept() which returns an error
    // when the socket is closed; we just flip the flag and let main exit.
    ctrlc_lite(move || {
        running_for_handler.store(false, Ordering::SeqCst);
        eprintln!("\n[serve] Ctrl-C received, shutting down");
    });

    let listener = match TcpListener::bind(BIND_ADDR) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[serve] bind {BIND_ADDR} failed: {e}");
            std::process::exit(1);
        }
    };

    // Short accept timeout so we can poll the running flag and exit cleanly.
    listener
        .set_nonblocking(false)
        .expect("set_nonblocking(false) cannot fail on a fresh listener");

    println!("[serve] http://{BIND_ADDR} — Ctrl-C to stop");
    println!("[serve] serving: examples/aphrody-terminal-demo.html ({} bytes)", DEMO_HTML.len());

    for incoming in listener.incoming() {
        if !running.load(Ordering::SeqCst) {
            break;
        }
        match incoming {
            Ok(stream) => {
                // One thread per connection. Demo traffic is trivially low;
                // a real server would use a pool, but we have no async runtime.
                thread::spawn(move || {
                    if let Err(e) = handle(stream) {
                        eprintln!("[serve] handler error: {e}");
                    }
                });
            }
            Err(e) => {
                eprintln!("[serve] accept error: {e}");
                thread::sleep(Duration::from_millis(50));
            }
        }
    }
}

/// Minimal HTTP/1.1 request parser + router. Supports GET only.
fn handle(mut stream: TcpStream) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;

    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;

    // Drain headers; we don't parse them but they must be consumed so
    // pipelined / keep-alive clients don't choke.
    let mut header_line = String::new();
    loop {
        header_line.clear();
        let n = reader.read_line(&mut header_line)?;
        if n == 0 || header_line == "\r\n" || header_line == "\n" {
            break;
        }
    }

    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("/");

    if method != "GET" {
        return write_response(&mut stream, 405, "Method Not Allowed", "text/plain", b"GET only\n");
    }

    match path {
        "/" | "/index.html" | "/aphrody-terminal-demo.html" => write_response(
            &mut stream,
            200,
            "OK",
            "text/html; charset=utf-8",
            DEMO_HTML.as_bytes(),
        ),
        "/favicon.ico" => {
            // 204 No Content — browsers stop asking after one round.
            stream.write_all(b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\n\r\n")?;
            stream.flush()
        }
        "/healthz" => write_response(
            &mut stream,
            200,
            "OK",
            "application/json",
            br#"{"status":"ok","service":"aphrody-wasm-serve","port":18800}"#,
        ),
        _ => write_response(&mut stream, 404, "Not Found", "text/plain", b"not found\n"),
    }
}

fn write_response(
    stream: &mut TcpStream,
    code: u16,
    reason: &str,
    content_type: &str,
    body: &[u8],
) -> std::io::Result<()> {
    let head = format!(
        "HTTP/1.1 {code} {reason}\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {len}\r\n\
         Cache-Control: no-cache\r\n\
         X-Aphrody-Demo: aphrody-terminal\r\n\
         Connection: close\r\n\
         \r\n",
        len = body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}

/// Best-effort shutdown notifier without pulling in the `ctrlc` crate.
///
/// On any platform, the OS will reap the process when the user sends real
/// SIGINT (Ctrl-C on Unix, generated console event on Windows). The accept
/// loop simply blocks until the kernel tears us down — `running` exists
/// purely so future iterations can plug in a richer signal handler
/// without changing the main loop's shape.
///
/// We intentionally do NOT read stdin: a background launch (no stdin TTY)
/// would otherwise hit immediate EOF and shut down on the first poll.
fn ctrlc_lite<F: Fn() + Send + Sync + 'static>(_cb: F) {
    // Intentional no-op. The accept loop's blocking `incoming()` will be
    // interrupted by SIGINT/CTRL_C_EVENT; the process exits before
    // user code observes a half-closed listener.
}

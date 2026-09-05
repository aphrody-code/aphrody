// SPDX-License-Identifier: Apache-2.0
//! Native PTY driver and WebSocket server.
//!
//! Uses [`portable_pty`] to open a pseudo-terminal on both Windows (ConPTY)
//! and Unix (POSIX openpty), then bridges it to a WebSocket connection so a
//! browser-based renderer can communicate with a real shell.
//!
//! # Wire protocol
//!
//! Each WebSocket message from **client to server** is one of:
//!
//! - **Binary frame** — raw bytes forwarded verbatim to the pty master writer.
//! - **Text frame** — a JSON control message:
//!   - `{"type":"resize","cols":N,"rows":N}` — resize the pty.
//!   - `{"type":"data","bytes":[...]}` — byte array forwarded to the pty.
//!
//! Each WebSocket message from **server to client** is:
//!
//! - **Binary frame** — raw bytes read from the pty master (shell output).

#![deny(unsafe_code)]

pub mod coord;
use std::{net::SocketAddr, path::PathBuf};

use anyhow::{Context as _, Result};
pub use coord::{
    CoordError, CoordProxy, CoordResponseKind, HttpResponse as CoordHttpResponse,
    NDJSON_CONTENT_TYPE, SSE_CONTENT_TYPE,
};
use futures_util::{SinkExt, stream::StreamExt};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use serde::Deserialize;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc,
};
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, info, warn};

// ── Configuration ─────────────────────────────────────────────────────────────

/// Configuration for a pty session.
#[derive(Debug, Clone)]
pub struct PtyConfig {
    /// Terminal width in columns.
    pub cols: u16,
    /// Terminal height in rows.
    pub rows: u16,
    /// Shell command to launch.  Defaults to the platform default shell when
    /// [`None`].
    pub shell: Option<String>,
    /// Working directory for the shell process.  Defaults to the current
    /// working directory when [`None`].
    pub cwd: Option<PathBuf>,
}

impl Default for PtyConfig {
    fn default() -> Self {
        Self { cols: 80, rows: 24, shell: None, cwd: None }
    }
}

/// Resolve the default shell for the current platform.
///
/// On Windows: tries `pwsh` first, falls back to `cmd`.
/// On Unix: uses `$SHELL` env var, falls back to `/bin/bash`.
pub fn default_shell() -> String {
    #[cfg(target_os = "windows")]
    {
        if which_shell("pwsh") {
            return "pwsh".to_owned();
        }
        "cmd".to_owned()
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(shell) = std::env::var("SHELL") {
            if !shell.is_empty() {
                return shell;
            }
        }
        "/bin/bash".to_owned()
    }
}

#[cfg(target_os = "windows")]
fn which_shell(name: &str) -> bool {
    use std::process::Command;
    Command::new("where").arg(name).output().map(|o| o.status.success()).unwrap_or(false)
}

// ── Control message ───────────────────────────────────────────────────────────

/// Structured control messages sent as JSON text frames from the client.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum ControlMsg {
    /// Resize the pty to new dimensions.
    Resize { cols: u16, rows: u16 },
    /// Send raw bytes to the pty (alternative to binary frames).
    Data { bytes: Vec<u8> },
}

// ── Server entry point ────────────────────────────────────────────────────────

/// Bind a TCP listener on `addr`, accept WebSocket connections, and spawn one
/// [`PtySession`] per connection.
///
/// This function runs indefinitely.  Cancel the surrounding task to shut down
/// the server.
pub async fn serve(addr: SocketAddr, cfg: PtyConfig) -> Result<()> {
    let listener = TcpListener::bind(addr).await.with_context(|| format!("bind {addr}"))?;
    info!("aphrody-terminal-backend listening on {addr}");

    // Send READY=1 to systemd (Type=notify)
    let _ = sd_notify("READY=1\nSTATUS=Ready to accept WebSocket terminal connections\n");
    // Start watchdog petting loop if watchdog health check is configured
    let _watchdog = start_watchdog_loop();

    loop {
        let (stream, peer) = listener.accept().await.context("accept")?;
        info!("new connection from {peer}");
        let cfg = cfg.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_conn(stream, cfg).await {
                warn!("session error: {e:#}");
            }
        });
    }
}

// ── Per-connection handler ────────────────────────────────────────────────────

/// Upgrade a raw TCP stream to WebSocket, open a pty, and bridge them.
async fn handle_conn(stream: TcpStream, cfg: PtyConfig) -> Result<()> {
    // WebSocket upgrade.
    let ws_stream = tokio_tungstenite::accept_async(stream).await.context("WebSocket handshake")?;
    let (mut ws_sink, mut ws_source) = ws_stream.split();

    // Open pty via portable_pty.
    let pty_system = native_pty_system();
    let pty_pair = pty_system
        .openpty(PtySize { rows: cfg.rows, cols: cfg.cols, pixel_width: 0, pixel_height: 0 })
        .context("openpty")?;

    // Build shell command.
    let shell = cfg.shell.clone().unwrap_or_else(default_shell);
    let mut cmd = CommandBuilder::new(&shell);
    if let Some(cwd) = cfg.cwd.clone() {
        cmd.cwd(cwd);
    }

    // Spawn the shell in the slave side of the pty.
    let mut child = pty_pair.slave.spawn_command(cmd).context("spawn shell")?;
    // The slave is consumed by the child; we keep master for I/O.
    drop(pty_pair.slave);
    let master = pty_pair.master;

    // Clone master handles for the two async tasks.
    // portable_pty reader/writer are synchronous — run them in blocking tasks.
    let mut master_reader = master.try_clone_reader().context("clone master reader")?;
    let master_writer = master.take_writer().context("take master writer")?;

    // Channel: pty bytes → ws sink.
    let (pty_tx, mut pty_rx) = mpsc::channel::<Vec<u8>>(256);

    // Task 1: read from pty master and forward to ws sink via channel.
    let reader_task = tokio::task::spawn_blocking(move || {
        let mut buf = [0u8; 4096];
        loop {
            match std::io::Read::read(&mut master_reader, &mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if pty_tx.blocking_send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                },
                Err(e) => {
                    debug!("pty read error: {e}");
                    break;
                },
            }
        }
    });

    // Task 2: forward pty bytes from channel to ws sink.
    let forward_task = tokio::spawn(async move {
        while let Some(data) = pty_rx.recv().await {
            if ws_sink.send(Message::Binary(data)).await.is_err() {
                break;
            }
        }
        // Attempt graceful WebSocket close.
        let _ = ws_sink.close().await;
    });

    // Task 3: read ws messages and forward to pty master writer.
    // The writer is synchronous; we wrap it in a blocking task via a channel.
    let (ws_to_pty_tx, mut ws_to_pty_rx) = mpsc::channel::<Vec<u8>>(256);
    let (resize_tx, mut resize_rx) = mpsc::channel::<PtySize>(16);

    let writer_task = tokio::task::spawn_blocking(move || {
        let mut writer = master_writer;
        loop {
            match ws_to_pty_rx.blocking_recv() {
                Some(data) => {
                    if std::io::Write::write_all(&mut writer, &data).is_err() {
                        break;
                    }
                },
                None => break,
            }
        }
    });

    // Read incoming WebSocket messages.
    while let Some(msg) = ws_source.next().await {
        let msg = match msg {
            Ok(m) => m,
            Err(e) => {
                debug!("ws recv error: {e}");
                break;
            },
        };

        match msg {
            Message::Binary(bytes) => {
                if ws_to_pty_tx.send(bytes).await.is_err() {
                    break;
                }
            },
            Message::Text(text) => match serde_json::from_str::<ControlMsg>(&text) {
                Ok(ControlMsg::Resize { cols, rows }) => {
                    let size = PtySize { rows, cols, pixel_width: 0, pixel_height: 0 };
                    if resize_tx.send(size).await.is_err() {
                        break;
                    }
                },
                Ok(ControlMsg::Data { bytes }) => {
                    if ws_to_pty_tx.send(bytes).await.is_err() {
                        break;
                    }
                },
                Err(e) => {
                    debug!("unrecognised control msg: {e}");
                },
            },
            Message::Close(_) => break,
            // Ping/Pong are handled by tungstenite automatically.
            _ => {},
        }
    }

    // Apply any pending resize requests in the resize channel.
    // (The resize task is the loop above; just drain the channel here.)
    drop(ws_to_pty_tx);
    // Apply any buffered resize messages after the ws loop ends.
    while let Ok(size) = resize_rx.try_recv() {
        if let Err(e) = master.resize(size) {
            debug!("resize error: {e}");
        }
    }

    // Wait for background tasks to finish.
    let _ = child.wait();
    let _ = reader_task.await;
    let _ = writer_task.await;
    let _ = forward_task.await;

    Ok(())
}

// ── systemd Notifications ───────────────────────────────────────────────────

/// Notify systemd of state changes (Type=notify). No-op on non-Unix platforms.
pub fn sd_notify(message: &str) -> std::io::Result<bool> {
    #[cfg(target_family = "unix")]
    {
        use std::env;
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;
        use std::os::unix::net::UnixDatagram;

        let socket_path = match env::var_os("NOTIFY_SOCKET") {
            Some(path) => path,
            None => return Ok(false),
        };

        let mut bytes = socket_path.into_vec();
        if !bytes.is_empty() && bytes[0] == b'@' {
            bytes[0] = b'\0';
        }
        let path = OsString::from_vec(bytes);

        let socket = UnixDatagram::unbound()?;
        socket.connect(path)?;
        socket.send(message.as_bytes())?;
        Ok(true)
    }
    #[cfg(not(target_family = "unix"))]
    {
        let _ = message;
        Ok(false)
    }
}

/// Spawns a background watchdog petting task if configured by systemd (WatchdogSec=...).
pub fn start_watchdog_loop() -> Option<tokio::task::JoinHandle<()>> {
    #[cfg(target_family = "unix")]
    {
        use std::env;
        use tracing::{error, trace};

        let watchdog_usec = match env::var("WATCHDOG_USEC") {
            Ok(val) => match val.parse::<u64>() {
                Ok(usec) => usec,
                Err(_) => return None,
            },
            Err(_) => return None,
        };

        if watchdog_usec == 0 {
            return None;
        }

        // Pet the watchdog at half the timeout interval
        let interval = std::time::Duration::from_micros(watchdog_usec / 2);
        info!("systemd watchdog enabled, petting every {}ms", interval.as_millis());

        Some(tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            // First tick fires immediately, skip it
            ticker.tick().await;

            loop {
                ticker.tick().await;
                trace!("petting systemd watchdog");
                if let Err(e) = sd_notify("WATCHDOG=1\n") {
                    error!("failed to pet systemd watchdog: {e}");
                }
            }
        }))
    }
    #[cfg(not(target_family = "unix"))]
    {
        None
    }
}

#[cfg(test)]
#[allow(unsafe_code)]
mod tests {
    use super::*;

    #[test]
    fn test_sd_notify_noop_when_no_env() {
        unsafe {
            std::env::remove_var("NOTIFY_SOCKET");
        }
        let res = sd_notify("READY=1");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), false);
    }

    #[cfg(target_family = "unix")]
    #[test]
    fn test_sd_notify_sends_datagram() {
        use std::os::unix::net::UnixDatagram;

        // Create a temporary socket to receive the notification
        let temp_dir = tempfile::tempdir().unwrap();
        let socket_path = temp_dir.path().join("notify.sock");
        let receiver = UnixDatagram::bind(&socket_path).unwrap();

        unsafe {
            std::env::set_var("NOTIFY_SOCKET", &socket_path);
        }
        let res = sd_notify("READY=1\n");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), true);

        let mut buf = [0u8; 100];
        let (len, _) = receiver.recv_from(&mut buf).unwrap();
        assert_eq!(&buf[..len], b"READY=1\n");

        unsafe {
            std::env::remove_var("NOTIFY_SOCKET");
        }
    }
}

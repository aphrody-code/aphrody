# systemd Integration Research Report

This document outlines the programmatic APIs, D-Bus control interfaces, and Rust integration patterns for systemd service management and watchdog/notifications within the Aphrody codebase.

---

## 1. Programmatic Daemon Notifications (`sd_notify`)

The `sd_notify` protocol is systemd's standard mechanism for daemon-to-manager notifications. It allows a service to communicate its lifecycle milestones, status updates, and health reports directly to systemd.

### 1.1 The Socket Mechanism

When a service unit defines `Type=notify` (or `Type=notify-reload`), systemd creates a Unix domain datagram socket and passes its path to the daemon via the `NOTIFY_SOCKET` environment variable.

* **Path Types:**
  * **Filesystem Socket:** E.g., `NOTIFY_SOCKET=/run/systemd/notify/socket`.
  * **Abstract Namespace Socket:** E.g., `NOTIFY_SOCKET=@/org/freedesktop/systemd1/notify`.
* **Abstract Namespace Handling:** Abstract sockets are Linux-specific. The `@` prefix is a conventional representation of a leading null byte (`\0`). When connecting to the socket path, the `@` prefix must be replaced with `\0` in memory to successfully bind or connect.

### 1.2 Message Format

Notifications are sent as newline-separated C-string variable assignments over the Unix datagram socket. Common states include:

| State Variable | Description |
| :--- | :--- |
| `READY=1` | Service initialization is complete and the service is fully operational. |
| `STATUS=...` | Free-form single-line status message displayed in `systemctl status`. |
| `WATCHDOG=1` | Watchdog keep-alive heartbeat (must be sent within the configured `WatchdogSec`). |
| `WATCHDOG=trigger` | Force-trigger a watchdog failure (useful when an internal deadlock is detected). |
| `STOPPING=1` | Service has started its graceful shutdown sequence. |
| `EXTEND_TIMEOUT_USEC=N` | Requests systemd to extend the current timeout by `N` microseconds. |

### 1.3 Socket Activation

Systemd supports socket activation where it opens sockets (TCP, UDP, Unix) on behalf of the service and hands over the file descriptors.
* **Environment Variables:**
  * `LISTEN_FDS`: Number of file descriptors passed.
  * `LISTEN_PID`: PID of the process that should receive them.
* **Constants:**
  * `SD_LISTEN_FDS_START = 3`: The first descriptor is always fd 3 (inherited stdout/stderr are 1/2).

---

## 2. D-Bus Control APIs (`org.freedesktop.systemd1`)

Systemd exposes a comprehensive D-Bus interface for programmatically querying and controlling services.

### 2.1 The System D-Bus

System-wide services are queried on the **System Bus** (`org.freedesktop.systemd1`). User-level systemd managers run on the **Session Bus**.

* **Service Name:** `org.freedesktop.systemd1`
* **Object Path:** `/org/freedesktop/systemd1`
* **Manager Interface:** `org.freedesktop.systemd1.Manager`

### 2.2 CLI Diagnostics (`busctl`)

D-Bus APIs can be queried and operated directly via the CLI for verification:

```bash
# Query general systemd properties
busctl get-property org.freedesktop.systemd1 /org/freedesktop/systemd1 org.freedesktop.systemd1.Manager Version Architecture SystemState

# Get the D-Bus object path for a specific service unit
busctl call org.freedesktop.systemd1 /org/freedesktop/systemd1 org.freedesktop.systemd1.Manager GetUnit s "aphrody.service"
# Returns: o "/org/freedesktop/systemd1/unit/aphrody_2eservice"

# Query the status of the service unit
busctl get-property org.freedesktop.systemd1 /org/freedesktop/systemd1/unit/aphrody_2eservice org.freedesktop.systemd1.Unit ActiveState SubState Description
# Returns:
# s "active"
# s "running"
# s "aphrody — static/SPA web server"
```

---

## 3. Rust Integration Patterns

There are three primary ways to integrate systemd notifications and D-Bus operations in a Rust project:

### 3.1 Pure Rust Zero-Dependency Notification (Selected Approach)

Instead of introducing external crates that complicate the dependency graph and require `cargo vet` supply-chain audits, we can implement the datagram socket protocol using Rust's standard library `std::os::unix::net::UnixDatagram`.

```rust
use std::env;
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::net::UnixDatagram;

pub fn notify_systemd(message: &str) -> std::io::Result<bool> {
    let socket_path = match env::var_os("NOTIFY_SOCKET") {
        Some(path) => path,
        None => return Ok(false), // No-op outside systemd
    };

    // Translate abstract socket '@' to '\0' in memory
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
```

### 3.2 High-Level Crate Integrations

* **`sd-notify` Crate:** A lightweight wrapper implementing readiness and watchdog notifications. Excellent but requires `cargo deny` / `vet` approval.
* **`zbus` Crate:** The standard crate for D-Bus integration in async Rust, offering macro-derived proxies:
  ```rust
  #[zbus::proxy(
      interface = "org.freedesktop.systemd1.Manager",
      default_service = "org.freedesktop.systemd1",
      default_path = "/org/freedesktop/systemd1"
  )]
  trait SystemdManager {
      async fn get_unit(&self, name: &str) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;
      async fn restart_unit(&self, name: &str, mode: &str) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;
  }
  ```

---

## 4. Aphrody Watchdog & Notification Implementation

In Aphrody, we implement systemd integration natively in `crates/aphrody-terminal-backend` and `crates/cli` using the zero-dependency standard library pattern.

### 4.1 readiness Notifications

We trigger `READY=1` when our long-running WebSocket terminal PTY bridge server (`aphrody term`) successfully binds to its TCP listener:

```rust
pub async fn serve(addr: SocketAddr, cfg: PtyConfig) -> Result<()> {
    let listener = TcpListener::bind(addr).await.with_context(|| format!("bind {addr}"))?;
    info!("aphrody-terminal-backend listening on {addr}");

    // Send READY=1 to systemd
    let _ = sd_notify("READY=1\nSTATUS=Ready to accept WebSocket terminal connections\n");
    
    // Start the systemd watchdog petting loop if configured
    let _watchdog = start_watchdog_loop();

    loop {
        // accept connections
    }
}
```

### 4.2 Watchdog Loop

If the systemd service file specifies a `WatchdogSec=N`, systemd will pass `WATCHDOG_USEC=M` (where `M = N * 1_000_000`) in the environment.
* The daemon must ping systemd by sending `WATCHDOG=1` at an interval strictly less than `WatchdogSec` (usually half the interval, `WatchdogSec / 2`).

We spawn an async watchdog task that:
1. Reads `WATCHDOG_USEC` from the environment.
2. Checks if `WATCHDOG_PID` matches our current process PID (optional validation).
3. Loops indefinitely, sleeping for `timeout / 2` and calling `sd_notify("WATCHDOG=1")`.

---

## 5. Deployment Unit File Example

To run `aphrody term` under systemd with notification and watchdog protection, use the following service template:

```ini
[Unit]
Description=Aphrody Terminal PTY WebSocket Server
After=network.target

[Service]
Type=notify
NotifyAccess=main
User=ubuntu
WorkingDirectory=/home/ubuntu/aphrody
ExecStart=/usr/local/bin/aphrody term --addr 127.0.0.1:8788
Restart=always
RestartSec=5

# Health monitoring watchdog
WatchdogSec=10s

# Sandboxing & Security
MemoryHigh=512M
MemoryMax=1G
PrivateTmp=true
ProtectSystem=full

[Install]
WantedBy=multi-user.target
```

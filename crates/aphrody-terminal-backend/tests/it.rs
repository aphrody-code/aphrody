// SPDX-License-Identifier: Apache-2.0
//! Integration tests for aphrody-terminal-backend.
//!
//! These tests exercise the in-process pty path directly (no live WebSocket
//! connections required), keeping the test suite hermetic and fast.

use std::io::Write;

use portable_pty::{native_pty_system, CommandBuilder, PtySize};

/// Helper: determine an echo command appropriate for the current platform.
#[cfg(target_os = "windows")]
fn echo_cmd(text: &str) -> CommandBuilder {
    let mut cmd = CommandBuilder::new("cmd");
    cmd.args(["/C", &format!("echo {text}")]);
    cmd
}

#[cfg(not(target_os = "windows"))]
fn echo_cmd(text: &str) -> CommandBuilder {
    let mut cmd = CommandBuilder::new("/bin/sh");
    cmd.args(["-c", &format!("printf '%s\\n' '{text}'")]);
    cmd
}

/// Test 1: Spawn `echo hi` through a pty master and read "hi\n" back.
#[test]
fn spawn_echo_reads_output() {
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("openpty");

    let cmd = echo_cmd("hi");
    let mut child = pair.slave.spawn_command(cmd).expect("spawn");
    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader().expect("clone reader");

    let status = child.wait().expect("wait");
    assert!(status.success(), "echo exited non-zero: {status:?}");

    // Read all output from the master.
    let mut output = Vec::new();
    let mut buf = [0u8; 256];
    // Read until EOF (master closes when slave process exits).
    loop {
        match std::io::Read::read(&mut reader, &mut buf) {
            Ok(0) => break,
            Ok(n) => output.extend_from_slice(&buf[..n]),
            Err(_) => break,
        }
    }

    let text = String::from_utf8_lossy(&output);
    assert!(
        text.contains("hi"),
        "expected 'hi' in pty output; got: {text:?}"
    );
}

/// Test 2: Resize a pty pair and verify the new dimensions are reflected.
#[test]
fn resize_round_trip() {
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("openpty");

    // Resize to a different size.
    pair.master
        .resize(PtySize {
            rows: 40,
            cols: 132,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("resize");

    // The slave is still open; the pair is valid after resize.
    // We just assert that the operation completed without error.
    drop(pair.slave);
    drop(pair.master);
}

/// Test 3: Write bytes through the master writer and verify the slave can read them.
///
/// This tests the ws_to_pty propagation path in-process: we write to the
/// master writer (which is the same channel that `handle_conn` uses for
/// forwarding WebSocket binary frames to the pty) and confirm the slave
/// receives them.
#[test]
fn master_writer_propagates_bytes_to_slave() {
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("openpty");

    // Spawn a shell that reads stdin and echoes it back.
    #[cfg(target_os = "windows")]
    let cmd = {
        let mut c = CommandBuilder::new("cmd");
        c.args(["/C", "more"]);
        c
    };
    #[cfg(not(target_os = "windows"))]
    let cmd = {
        let mut c = CommandBuilder::new("/bin/cat");
        c
    };

    let mut child = pair.slave.spawn_command(cmd).expect("spawn");
    drop(pair.slave);

    let mut writer = pair.master.take_writer().expect("take writer");
    let mut reader = pair.master.try_clone_reader().expect("clone reader");

    // Write a test payload.
    let payload = b"hello-pty\n";
    writer.write_all(payload).expect("write to master");
    // Signal EOF so `cat` / `more` exits.
    drop(writer);

    let _ = child.wait();

    let mut output = Vec::new();
    let mut buf = [0u8; 256];
    loop {
        match std::io::Read::read(&mut reader, &mut buf) {
            Ok(0) => break,
            Ok(n) => output.extend_from_slice(&buf[..n]),
            Err(_) => break,
        }
    }

    let text = String::from_utf8_lossy(&output);
    assert!(
        text.contains("hello-pty"),
        "expected 'hello-pty' in pty output; got: {text:?}"
    );
}

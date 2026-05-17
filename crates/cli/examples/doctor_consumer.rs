// SPDX-License-Identifier: Apache-2.0
//
// Example: consume `aphrody doctor --json` output programmatically.
//
// Usage:
//   cargo run -p aphrody --example doctor_consumer
//
// What it does:
//   1. Invokes the aphrody binary in-process via std::process::Command.
//   2. Parses the JSON output.
//   3. Renders a 1-line human-readable verdict.
//   4. Exits 0 if HEALTHY, 1 if DEGRADED, 2 if BROKEN.

use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_aphrody"))
        .args(["doctor", "--json"])
        .output()?;

    if !output.status.success() {
        eprintln!("aphrody doctor exited with status {}", output.status);
        std::process::exit(3);
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;

    let verdict = json
        .get("verdict")
        .and_then(|v| v.as_str())
        .ok_or("missing verdict field")?;

    let peer_status = json
        .get("peer_a2a")
        .and_then(|p| p.get("http_listener_status"))
        .and_then(|s| s.as_u64())
        .unwrap_or(0);

    let heartbeat_age = json
        .get("peer_a2a")
        .and_then(|p| p.get("heartbeat_age_s"))
        .and_then(|a| a.as_u64());

    println!(
        "verdict={verdict} listener={peer_status} heartbeat_age={age}",
        verdict = verdict,
        peer_status = peer_status,
        age = heartbeat_age.map_or_else(|| "n/a".to_owned(), |a| a.to_string())
    );

    match verdict {
        "HEALTHY" => std::process::exit(0),
        "DEGRADED" => std::process::exit(1),
        "BROKEN" => std::process::exit(2),
        _ => {
            eprintln!("unknown verdict: {verdict}");
            std::process::exit(4)
        }
    }
}

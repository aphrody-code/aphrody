// SPDX-License-Identifier: Apache-2.0
//
// bxc-engine-daemon — HTTP API binary serving the bxc /api/* contract.
//
// Standalone entry point so `aphrody bxc daemon` can spawn it without
// pulling in the CDP-server-only `bxc-engine` main binary. Routes are
// implemented in `bxc_engine::daemon`.
//
// Usage:
//   bxc-engine-daemon                  # bind 127.0.0.1:8765
//   bxc-engine-daemon --port 9000      # custom port
//   bxc-engine-daemon --host 0.0.0.0   # all interfaces (Docker)
//
// Honoured env :
//   BXC_DAEMON_HOST   (default 127.0.0.1)
//   BXC_DAEMON_PORT   (default 8765)

use std::net::{IpAddr, SocketAddr};

use bxc_engine::daemon;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    // Install rustls provider once for any downstream reqwest HTTPS calls.
    let _ = rustls::crypto::ring::default_provider().install_default();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();

    let (host, port) = parse_args();
    let ip: IpAddr = host
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid host `{host}`: {e}"))?;
    let addr = SocketAddr::new(ip, port);
    println!("[bxc-engine-daemon] http://{addr}");
    daemon::serve(addr).await
}

fn parse_args() -> (String, u16) {
    let mut host = std::env::var("BXC_DAEMON_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let mut port: u16 = std::env::var("BXC_DAEMON_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8765);

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--port" | "-p" => {
                if let Some(v) = args.get(i + 1) {
                    if let Ok(p) = v.parse::<u16>() {
                        port = p;
                    }
                    i += 1;
                }
            }
            "--host" => {
                if let Some(v) = args.get(i + 1) {
                    host = v.clone();
                    i += 1;
                }
            }
            "--help" | "-h" => {
                eprintln!(
                    "bxc-engine-daemon — bxc /api/* HTTP server

USAGE:
  bxc-engine-daemon [--host <h>] [--port <p>]

ENV:
  BXC_DAEMON_HOST (default 127.0.0.1)
  BXC_DAEMON_PORT (default 8765)

ROUTES:
  GET    /healthz
  GET|POST /api/recon
  GET|POST /api/detect
  GET|POST /api/scrape
  GET    /api/tokens"
                );
                std::process::exit(0);
            }
            _ => {}
        }
        i += 1;
    }
    (host, port)
}

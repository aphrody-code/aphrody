// SPDX-License-Identifier: Apache-2.0
//! Standalone entrypoint for the `aphrody-serve` OpenAI-compatible server.
//!
//! This binary is the thin CLI wrapper around the [`aphrody_serve`] library;
//! the `aphrody serve` subcommand calls the same `serve`/`local_state` API.

use std::net::{IpAddr, SocketAddr};

use clap::Parser;

/// OpenAI-compatible server for local open-weight models.
#[derive(Debug, Parser)]
#[command(name = "aphrody-serve", version, about)]
struct Args {
    /// Host/IP to bind.
    #[arg(long, default_value = "127.0.0.1", env = "APHRODY_SERVE_HOST")]
    host: IpAddr,

    /// TCP port to bind.
    #[arg(long, default_value_t = 8080, env = "APHRODY_SERVE_PORT")]
    port: u16,

    /// Backend root URL of the OpenAI-compatible engine (Ollama, llama.cpp,
    /// vLLM …). The adapter appends `/v1/chat/completions`.
    #[arg(long, default_value = aphrody_serve::DEFAULT_BACKEND_URL, env = "OPENAI_BASE_URL")]
    base_url: String,

    /// Bearer token forwarded to the backend (ignored by Ollama).
    #[arg(long, default_value = "ollama", env = "OPENAI_API_KEY")]
    api_key: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let args = Args::parse();
    let addr = SocketAddr::new(args.host, args.port);
    let state = aphrody_serve::local_state(&args.base_url, &args.api_key)?;

    tracing::info!(backend = %args.base_url, "starting aphrody-serve");
    serve_or_explain(addr, state, args.port).await
}

/// Runs the server, turning an address-in-use bind failure into a clear hint.
async fn serve_or_explain(
    addr: SocketAddr,
    state: aphrody_serve::AppState,
    port: u16,
) -> anyhow::Result<()> {
    if let Err(e) = aphrody_serve::serve(addr, state).await {
        if e.kind() == std::io::ErrorKind::AddrInUse {
            anyhow::bail!("port {port} is already in use — pass --port to choose another");
        }
        return Err(e.into());
    }
    Ok(())
}

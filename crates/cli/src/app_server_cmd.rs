// SPDX-License-Identifier: Apache-2.0
//! CLI entry point for Aphrody's local Codex-shaped app-server.

use aphrody_app_server::{serve_http, serve_stdio};
use tokio::io::{BufReader, stdin, stdout};

use crate::SessionAction;

pub(crate) async fn run(action: SessionAction) -> miette::Result<()> {
    match action {
        SessionAction::Serve { http, addr } => {
            if http {
                let addr = addr
                    .parse()
                    .map_err(|error| miette::miette!("app-server: invalid address: {error}"))?;
                serve_http(addr).await.map_err(|error| miette::miette!("app-server: {error}"))?;
            } else {
                serve_stdio(BufReader::new(stdin()), stdout())
                    .await
                    .map_err(|error| miette::miette!("app-server: {error}"))?;
            }
        },
    }
    Ok(())
}

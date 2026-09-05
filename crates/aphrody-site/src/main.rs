// SPDX-License-Identifier: Apache-2.0

//! Minimal public origin server for aphrody.com.

use std::{env, net::SocketAddr};

use axum::{
    Router,
    http::{HeaderValue, StatusCode, header},
    response::{Html, IntoResponse},
    routing::get,
};
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::EnvFilter;

const INDEX: &str = "<!doctype html><html lang=\"fr\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><meta name=\"theme-color\" content=\"#fff\"><link rel=\"canonical\" href=\"https://aphrody.com/\"><title>Aphrody</title></head><body></body></html>";
const SECURITY: &str = "Contact: mailto:contact@aphrody.com\nCanonical: https://aphrody.com/.well-known/security.txt\nPreferred-Languages: fr, en\n";

#[tokio::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .json()
        .init();

    let bind = env::var("APHRODY_SITE_BIND").unwrap_or_else(|_| "127.0.0.1:8083".to_owned());
    let address: SocketAddr = bind.parse().map_err(std::io::Error::other)?;
    let listener = TcpListener::bind(address).await?;
    info!(%address, "aphrody.com origin ready");

    axum::serve(listener, app()).await
}

fn app() -> Router {
    Router::new()
        .route("/", get(index))
        .route("/healthz", get(health))
        .route("/robots.txt", get(robots))
        .route("/.well-known/security.txt", get(security))
        .fallback(not_found)
}

async fn index() -> impl IntoResponse {
    (
        [
            (header::CACHE_CONTROL, HeaderValue::from_static("public, max-age=300")),
            (
                header::CONTENT_SECURITY_POLICY,
                HeaderValue::from_static(
                    "default-src 'none'; base-uri 'none'; frame-ancestors 'none'",
                ),
            ),
            (header::REFERRER_POLICY, HeaderValue::from_static("no-referrer")),
            (header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff")),
        ],
        Html(INDEX),
    )
}

async fn health() -> &'static str {
    "ok\n"
}

async fn robots() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], "User-agent: *\nAllow: /\n")
}

async fn security() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], SECURITY)
}

async fn not_found() -> StatusCode {
    StatusCode::NOT_FOUND
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_page_has_no_visible_body() {
        assert!(INDEX.contains("<body></body>"));
    }

    #[test]
    fn security_contact_uses_domain() {
        assert!(SECURITY.contains("contact@aphrody.com"));
    }
}

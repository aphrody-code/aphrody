// SPDX-License-Identifier: Apache-2.0
//
// RFC 7591 — OAuth 2.0 Dynamic Client Registration Protocol.
//
// Implements the full registration flow plus a persistent JSON cache stored
// in `var/data/mcp-oauth-clients.json` (relative to the caller-supplied
// `data_dir`).  The cache is keyed by `(auth_server_issuer, redirect_uri)`,
// matching the upstream byte-for-byte semantics in the reference TypeScript.
//
// Atomic writes: content is written to a `.tmp` file first, then renamed, so
// a crash mid-write never corrupts the cache.

use std::path::{Path, PathBuf};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::{Error, discovery::AuthorizationServerMetadata};

// ── Cache file types ─────────────────────────────────────────────────────────

/// A registered OAuth client credential pair.  Persisted to disk so we
/// register once and reuse forever (RFC 7591 §3.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredClient {
    /// Issuer URI of the authorization server that issued this client_id.
    pub auth_server_issuer: String,
    /// Redirect URI that was registered.
    pub redirect_uri: String,
    /// Issued `client_id`.
    pub client_id: String,
    /// Issued `client_secret`, present only for confidential clients.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,
    /// Unix timestamp (seconds) at which this registration was stored.
    pub registered_at: u64,
}

/// On-disk cache file structure.
#[derive(Debug, Default, Serialize, Deserialize)]
struct ClientCacheFile {
    clients: Vec<RegisteredClient>,
}

// ── Raw registration (RFC 7591) ──────────────────────────────────────────────

/// POST to the authorization server's `registration_endpoint` per RFC 7591.
///
/// Sends the minimal client metadata required for the MCP OAuth 2.1 flow:
///   - `redirect_uris`
///   - `token_endpoint_auth_method = "none"` (public client / PKCE-only)
///   - `grant_types = ["authorization_code", "refresh_token"]`
///   - `response_types = ["code"]`
///   - `client_name = "Aphrody"`
///   - `application_type = "web"`
///
/// # Errors
///
/// Returns `Err(Error::RegistrationFailed)` if the server returns a non-2xx
/// status or if `client_id` is absent from the response body.
pub async fn register_client_raw(
    client: &Client,
    registration_endpoint: &str,
    redirect_uri: &str,
) -> Result<(String, Option<String>), Error> {
    #[derive(Serialize)]
    struct RegistrationBody<'a> {
        redirect_uris: [&'a str; 1],
        token_endpoint_auth_method: &'static str,
        grant_types: [&'static str; 2],
        response_types: [&'static str; 1],
        client_name: &'static str,
        application_type: &'static str,
    }

    #[derive(Deserialize)]
    struct RegistrationResponse {
        client_id: Option<String>,
        client_secret: Option<String>,
    }

    let body = RegistrationBody {
        redirect_uris: [redirect_uri],
        token_endpoint_auth_method: "none",
        grant_types: ["authorization_code", "refresh_token"],
        response_types: ["code"],
        client_name: "Aphrody",
        application_type: "web",
    };

    let resp = client
        .post(registration_endpoint)
        .json(&body)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await
        .map_err(|e| Error::Http { url: registration_endpoint.to_owned(), source: e })?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body_text = resp.text().await.unwrap_or_default();
        let snippet = body_text.chars().take(500).collect::<String>();
        return Err(Error::RegistrationFailed {
            endpoint: registration_endpoint.to_owned(),
            status,
            body: snippet,
        });
    }

    let reg: RegistrationResponse = resp
        .json()
        .await
        .map_err(|e| Error::Http { url: registration_endpoint.to_owned(), source: e })?;

    let client_id = reg.client_id.ok_or_else(|| Error::RegistrationFailed {
        endpoint: registration_endpoint.to_owned(),
        status: 0,
        body: "response missing client_id".to_owned(),
    })?;

    debug!(endpoint = registration_endpoint, client_id, "dynamic client registration succeeded");

    Ok((client_id, reg.client_secret))
}

// ── Cached registration ──────────────────────────────────────────────────────

/// Return a cached `RegisteredClient` for `(auth_server.issuer, redirect_uri)`,
/// or perform a fresh RFC 7591 registration and persist the result.
///
/// Cache file location: `<data_dir>/mcp-oauth-clients.json`.
///
/// # Errors
///
/// - `Error::NoRegistrationEndpoint` when the server does not publish one and no cached client
///   exists.
/// - `Error::CacheIo` on filesystem read/write failures.
/// - `Error::RegistrationFailed` on non-2xx from the registration endpoint.
pub async fn get_or_register_client(
    http: &Client,
    data_dir: &Path,
    auth_server: &AuthorizationServerMetadata,
    redirect_uri: &str,
) -> Result<RegisteredClient, Error> {
    let mut cache = read_cache(data_dir).await?;

    // Cache hit: return existing client for this (issuer, redirect_uri) pair.
    if let Some(existing) = cache
        .clients
        .iter()
        .find(|c| c.auth_server_issuer == auth_server.issuer && c.redirect_uri == redirect_uri)
    {
        debug!(
            issuer = %auth_server.issuer,
            redirect_uri,
            client_id = %existing.client_id,
            "using cached OAuth client registration"
        );
        return Ok(existing.clone());
    }

    // Cache miss: need a registration endpoint.
    let endpoint = auth_server
        .registration_endpoint
        .as_deref()
        .ok_or_else(|| Error::NoRegistrationEndpoint { issuer: auth_server.issuer.clone() })?;

    info!(
        issuer = %auth_server.issuer,
        redirect_uri,
        "performing dynamic client registration (RFC 7591)"
    );

    let (client_id, client_secret) = register_client_raw(http, endpoint, redirect_uri).await?;

    let registered_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let registered = RegisteredClient {
        auth_server_issuer: auth_server.issuer.clone(),
        redirect_uri: redirect_uri.to_owned(),
        client_id,
        client_secret,
        registered_at,
    };

    cache.clients.push(registered.clone());
    write_cache(data_dir, &cache).await?;

    Ok(registered)
}

// ── Cache I/O helpers ────────────────────────────────────────────────────────

fn cache_path(data_dir: &Path) -> PathBuf {
    data_dir.join("mcp-oauth-clients.json")
}

async fn read_cache(data_dir: &Path) -> Result<ClientCacheFile, Error> {
    let path = cache_path(data_dir);
    match tokio::fs::read_to_string(&path).await {
        Ok(raw) => match serde_json::from_str::<ClientCacheFile>(&raw) {
            Ok(f) => Ok(f),
            Err(e) => {
                warn!(
                    path = %path.display(),
                    err = %e,
                    "cache file corrupt — starting fresh"
                );
                Ok(ClientCacheFile::default())
            },
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(ClientCacheFile::default()),
        Err(e) => Err(Error::CacheIo { path: path.to_string_lossy().into_owned(), source: e }),
    }
}

/// Atomically overwrite the cache file using a `.tmp` rename.
async fn write_cache(data_dir: &Path, cache: &ClientCacheFile) -> Result<(), Error> {
    let path = cache_path(data_dir);

    // Ensure parent directory exists.
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|e| Error::CacheIo {
            path: parent.to_string_lossy().into_owned(),
            source: e,
        })?;
    }

    let json = serde_json::to_string_pretty(cache).map_err(|e| Error::CacheIo {
        path: path.to_string_lossy().into_owned(),
        source: std::io::Error::new(std::io::ErrorKind::InvalidData, e),
    })?;

    // Write to a sibling .tmp file, then atomically rename.
    let tmp = path.with_extension("tmp");
    tokio::fs::write(&tmp, json.as_bytes())
        .await
        .map_err(|e| Error::CacheIo { path: tmp.to_string_lossy().into_owned(), source: e })?;

    tokio::fs::rename(&tmp, &path)
        .await
        .map_err(|e| Error::CacheIo { path: path.to_string_lossy().into_owned(), source: e })?;

    debug!(path = %path.display(), "OAuth client cache written");
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    fn make_client() -> RegisteredClient {
        RegisteredClient {
            auth_server_issuer: "https://auth.example.com".to_owned(),
            redirect_uri: "https://app.example.com/callback".to_owned(),
            client_id: "test-client-001".to_owned(),
            client_secret: Some("s3cr3t".to_owned()),
            registered_at: 1_700_000_000,
        }
    }

    #[test]
    fn registered_client_serde_roundtrip() {
        let original = make_client();
        let json = serde_json::to_string(&original).unwrap();
        let decoded: RegisteredClient = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.client_id, original.client_id);
        assert_eq!(decoded.auth_server_issuer, original.auth_server_issuer);
        assert_eq!(decoded.redirect_uri, original.redirect_uri);
        assert_eq!(decoded.client_secret, original.client_secret);
        assert_eq!(decoded.registered_at, original.registered_at);
    }

    #[test]
    fn registered_client_no_secret_omitted_in_json() {
        let mut c = make_client();
        c.client_secret = None;
        let json = serde_json::to_string(&c).unwrap();
        // `client_secret` must be absent when None (skip_serializing_if).
        assert!(!json.contains("client_secret"));
    }

    #[tokio::test]
    async fn cache_roundtrip_and_empty_on_missing_file() {
        let dir = TempDir::new().unwrap();
        let data_dir = dir.path();

        // Cold read: no file yet — returns empty cache.
        let cache = read_cache(data_dir).await.unwrap();
        assert!(cache.clients.is_empty());

        // Write a client then read it back.
        let mut to_write = ClientCacheFile::default();
        to_write.clients.push(make_client());
        write_cache(data_dir, &to_write).await.unwrap();

        let read_back = read_cache(data_dir).await.unwrap();
        assert_eq!(read_back.clients.len(), 1);
        assert_eq!(read_back.clients[0].client_id, "test-client-001");
    }
}

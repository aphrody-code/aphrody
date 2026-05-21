// SPDX-License-Identifier: Apache-2.0
//! Local Codeium `language_server.exe` gRPC bridge (feature `local-ls`).
//!
//! The Antigravity desktop shell (Electron) supervises a Codeium Go
//! `language_server` binary that owns all cloud auth and exposes a self-signed,
//! cert-pinned **HTTPS gRPC** endpoint on `127.0.0.1`.  This module reproduces
//! that exact launch contract so aphrody can talk to the same service directly,
//! without the proprietary Electron shell.
//!
//! ## Host-only, opt-in
//!
//! This module is gated behind the non-default `local-ls` cargo feature and is
//! never compiled in the default build, CI, or wasm32 targets.  It also
//! requires the proprietary `language_server` binary to be present at runtime
//! (resolved via [`LanguageServer::resolve_binary`]); nothing here embeds it.
//!
//! ## Launch contract (verbatim from the shipped `languageServer.js`)
//!
//! ```text
//! language_server --standalone
//!   --override_ide_name <name> --subclient_type hub
//!   --override_ide_version <v> --override_user_agent_name <name>
//!   --https_server_port <port|0> --csrf_token <csrf>
//!   --app_data_dir <dir>
//!   --api_server_url https://generativelanguage.googleapis.com
//!   --cloud_code_endpoint https://daily-cloudcode-pa.googleapis.com
//!   --enable_sidecars [--headless]
//! ```
//!
//! The bound port is printed on stdout / the log, matched by the regex
//! `listening on \w+ port at (\d+) for HTTP(S)?` (case-insensitive).
//!
//! ## Transport security
//!
//! The local endpoint serves a **self-signed** certificate, so the standard
//! WebPKI chain check cannot apply.  Instead we pin the certificate's
//! `SubjectPublicKeyInfo` SHA-256 to the constant
//! [`PINNED_SPKI_SHA256_B64`] (Chromium HPKP `sha256/…` format) via a custom
//! [`rustls::client::danger::ServerCertVerifier`].  A CSRF token (generated per
//! launch, or supplied) is attached to every gRPC call.

use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use rand::RngCore;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, Error as RustlsError, SignatureScheme};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tonic::transport::{Channel, Endpoint};
use tracing::{debug, warn};

use crate::error::SdkError;

/// Generated tonic client + prost message types for
/// `exa.language_server_pb` (compiled by `build.rs` into `OUT_DIR`).
#[allow(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    missing_docs,
    rustdoc::all
)]
pub mod proto {
    tonic::include_proto!("exa.language_server_pb");
}

use proto::language_server_service_client::LanguageServerServiceClient;
use proto::{
    FetchUserInfoRequest, FetchUserInfoResponse, GetAuthStatusRequest, GetAuthStatusResponse,
    GetAvailableModelsRequest, GetAvailableModelsResponse,
};

/// Pinned `SubjectPublicKeyInfo` SHA-256 (Chromium `sha256/<base64>` format),
/// recovered from the Antigravity 2.0.1 `constants.js`.  This is the public
/// fingerprint of the language server's self-signed certificate — not a secret.
pub const PINNED_SPKI_SHA256_B64: &str = "sTZpQemOWEytaZqa7P/y/dNXbHMdOAzMvzHEhUwHZXw=";

/// Environment variable that overrides the language server binary path (Antigravity harness).
pub const ENV_ANTIGRAVITY_HARNESS: &str = "ANTIGRAVITY_HARNESS_PATH";

/// Environment variable that overrides the language server binary path (legacy Codeium).
pub const ENV_LANGUAGE_SERVER_BIN: &str = "CODEIUM_LANGUAGE_SERVER_BIN";

/// Default upstream API server URL passed to the language server.
pub const DEFAULT_API_SERVER_URL: &str = "https://generativelanguage.googleapis.com";

/// Default Cloud Code endpoint passed to the language server (the shell's
/// runtime default is the `daily` ring).
pub const DEFAULT_CLOUD_CODE_ENDPOINT: &str = "https://daily-cloudcode-pa.googleapis.com";

/// Header name carrying the CSRF token on every gRPC call.
const CSRF_HEADER: &str = "x-csrf-token";

/// How long to wait for the language server to print its bound port.
const DEFAULT_PORT_TIMEOUT: Duration = Duration::from_secs(20);

/// Configuration for launching the local language server.
///
/// Construct with [`LaunchConfig::new`] and tweak fields as needed.  All string
/// fields map 1:1 onto the documented launch flags.
#[derive(Debug, Clone)]
pub struct LaunchConfig {
    /// `--override_ide_name` (also reused for `--override_user_agent_name`).
    pub ide_name: String,
    /// `--override_ide_version`.
    pub ide_version: String,
    /// `--app_data_dir` — directory the server uses for its local state.
    pub app_data_dir: String,
    /// `--api_server_url`.
    pub api_server_url: String,
    /// `--cloud_code_endpoint`.
    pub cloud_code_endpoint: String,
    /// `--https_server_port` (`0` lets the OS choose a free port).
    pub https_server_port: u16,
    /// CSRF token sent on every call. Generated when [`None`].
    pub csrf_token: Option<String>,
    /// Append `--headless` to the argument list.
    pub headless: bool,
    /// Maximum time to wait for the port line on stdout.
    pub port_timeout: Duration,
}

impl LaunchConfig {
    /// Create a config with the given IDE identity and app-data directory, the
    /// documented default endpoints, OS-chosen port, and a freshly generated
    /// CSRF token.
    #[must_use]
    pub fn new(
        ide_name: impl Into<String>,
        ide_version: impl Into<String>,
        app_data_dir: impl Into<String>,
    ) -> Self {
        Self {
            ide_name: ide_name.into(),
            ide_version: ide_version.into(),
            app_data_dir: app_data_dir.into(),
            api_server_url: DEFAULT_API_SERVER_URL.to_string(),
            cloud_code_endpoint: DEFAULT_CLOUD_CODE_ENDPOINT.to_string(),
            https_server_port: 0,
            csrf_token: None,
            headless: true,
            port_timeout: DEFAULT_PORT_TIMEOUT,
        }
    }

    /// Resolve the effective CSRF token, generating a random 256-bit
    /// hex token when none was supplied.
    fn resolved_csrf(&self) -> String {
        self.csrf_token.clone().unwrap_or_else(generate_csrf_token)
    }
}

/// Generate a random 256-bit CSRF token, hex-encoded.
fn generate_csrf_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    let mut out = String::with_capacity(64);
    for b in bytes {
        use std::fmt::Write as _;
        let _ = write!(out, "{b:02x}");
    }
    out
}

/// A spawned local language server plus a connected gRPC channel.
///
/// Dropping the value kills the child process.
pub struct LanguageServer {
    child: Child,
    port: u16,
    csrf: String,
    channel: Channel,
}

impl LanguageServer {
    /// Resolve the language server binary path.
    ///
    /// Resolution order:
    /// 1. `$CODEIUM_LANGUAGE_SERVER_BIN`,
    /// 2. `resources/bin/language_server[.exe]` next to the current executable,
    /// 3. `language_server[.exe]` on `PATH` (returned verbatim; let the OS
    ///    resolve it at spawn time).
    ///
    /// # Errors
    ///
    /// Returns [`SdkError::LanguageServerNotFound`] when no candidate exists.
    pub fn resolve_binary() -> Result<std::path::PathBuf, SdkError> {
        // 1. Check ANTIGRAVITY_HARNESS_PATH first (Antigravity standard)
        if let Some(p) = std::env::var_os(ENV_ANTIGRAVITY_HARNESS) {
            let path = std::path::PathBuf::from(p);
            if path.is_file() {
                return Ok(path);
            }
            // If it's a directory, try appending known binary names.
            let exe_names = if cfg!(windows) {
                &["language_server.exe", "localharness.exe"][..]
            } else {
                &["language_server", "localharness"][..]
            };
            for name in exe_names {
                let candidate = path.join(name);
                if candidate.is_file() {
                    return Ok(candidate);
                }
            }
            warn!(?path, "ANTIGRAVITY_HARNESS_PATH set but no valid binary found");
        }

        // 2. Check legacy CODEIUM_LANGUAGE_SERVER_BIN
        if let Some(p) = std::env::var_os(ENV_LANGUAGE_SERVER_BIN) {
            let path = std::path::PathBuf::from(p);
            if path.is_file() {
                return Ok(path);
            }
            warn!(?path, "CODEIUM_LANGUAGE_SERVER_BIN set but not a file");
        }

        let exe_name = if cfg!(windows) {
            "language_server.exe"
        } else {
            "language_server"
        };

        if let Ok(current) = std::env::current_exe()
            && let Some(dir) = current.parent()
        {
            let candidate = dir.join("resources").join("bin").join(exe_name);
            if candidate.is_file() {
                return Ok(candidate);
            }
        }

        // Fall back to PATH resolution at spawn time.
        Ok(std::path::PathBuf::from(exe_name))
    }

    /// Print the language server build CL by running it with `--stamp`.
    ///
    /// # Errors
    ///
    /// Returns [`SdkError::Spawn`] when the binary cannot be executed.
    pub async fn stamp() -> Result<String, SdkError> {
        let bin = Self::resolve_binary()?;
        let output = Command::new(&bin)
            .arg("--stamp")
            .output()
            .await
            .map_err(|e| SdkError::Spawn(e.to_string()))?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Spawn the language server with the documented launch contract, wait for
    /// it to print its bound port, and build a cert-pinned gRPC channel.
    ///
    /// # Errors
    ///
    /// * [`SdkError::Spawn`] — the binary could not be launched or stdout was
    ///   not captured.
    /// * [`SdkError::PortTimeout`] — the port line did not appear in time.
    /// * [`SdkError::Tls`] — the pinned rustls config could not be built.
    /// * [`SdkError::Http`] / [`SdkError::Transport`] — the channel could not
    ///   be established.
    pub async fn launch(config: &LaunchConfig) -> Result<Self, SdkError> {
        let bin = Self::resolve_binary()?;
        let csrf = config.resolved_csrf();

        let mut cmd = Command::new(&bin);
        cmd.arg("--standalone")
            .arg("--override_ide_name")
            .arg(&config.ide_name)
            .arg("--subclient_type")
            .arg("hub")
            .arg("--override_ide_version")
            .arg(&config.ide_version)
            .arg("--override_user_agent_name")
            .arg(&config.ide_name)
            .arg("--https_server_port")
            .arg(config.https_server_port.to_string())
            .arg("--csrf_token")
            .arg(&csrf)
            .arg("--app_data_dir")
            .arg(&config.app_data_dir)
            .arg("--api_server_url")
            .arg(&config.api_server_url)
            .arg("--cloud_code_endpoint")
            .arg(&config.cloud_code_endpoint)
            .arg("--enable_sidecars");
        if config.headless {
            cmd.arg("--headless");
        }
        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .kill_on_drop(true);

        debug!(?bin, "spawning language_server");
        let mut child = cmd.spawn().map_err(|e| SdkError::Spawn(e.to_string()))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| SdkError::Spawn("language_server stdout not captured".into()))?;

        let port = tokio::time::timeout(config.port_timeout, read_port(stdout))
            .await
            .map_err(|_| SdkError::PortTimeout)??;

        debug!(port, "language_server listening");

        let channel = build_pinned_channel(port).await?;

        Ok(Self {
            child,
            port,
            csrf,
            channel,
        })
    }

    /// The TCP port the language server bound to.
    #[must_use]
    pub fn port(&self) -> u16 {
        self.port
    }

    /// The CSRF token attached to every gRPC call.
    #[must_use]
    pub fn csrf(&self) -> &str {
        &self.csrf
    }

    /// A clone of the underlying cert-pinned gRPC channel.
    #[must_use]
    pub fn channel(&self) -> Channel {
        self.channel.clone()
    }

    /// Build a `LanguageServerService` gRPC client with the CSRF interceptor
    /// applied to every request.
    fn client(
        &self,
    ) -> LanguageServerServiceClient<
        tonic::service::interceptor::InterceptedService<Channel, CsrfInterceptor>,
    > {
        let interceptor = CsrfInterceptor {
            csrf: self.csrf.clone(),
        };
        LanguageServerServiceClient::with_interceptor(self.channel.clone(), interceptor)
    }

    /// Call `FetchUserInfo`.
    ///
    /// # Errors
    ///
    /// Returns [`SdkError::Grpc`] on a gRPC-level failure.
    pub async fn fetch_user_info(&self) -> Result<FetchUserInfoResponse, SdkError> {
        let mut client = self.client();
        let resp = client
            .fetch_user_info(FetchUserInfoRequest::default())
            .await
            .map_err(SdkError::from)?;
        Ok(resp.into_inner())
    }

    /// Call `GetAuthStatus`.
    ///
    /// # Errors
    ///
    /// Returns [`SdkError::Grpc`] on a gRPC-level failure.
    pub async fn get_auth_status(&self) -> Result<GetAuthStatusResponse, SdkError> {
        let mut client = self.client();
        let resp = client
            .get_auth_status(GetAuthStatusRequest::default())
            .await
            .map_err(SdkError::from)?;
        Ok(resp.into_inner())
    }

    /// Call `GetAvailableModels`.
    ///
    /// # Errors
    ///
    /// Returns [`SdkError::Grpc`] on a gRPC-level failure.
    pub async fn get_available_models(&self) -> Result<GetAvailableModelsResponse, SdkError> {
        let mut client = self.client();
        let resp = client
            .get_available_models(GetAvailableModelsRequest::default())
            .await
            .map_err(SdkError::from)?;
        Ok(resp.into_inner())
    }

    /// Best-effort terminate the child process.
    ///
    /// `Drop` already kills the child (`kill_on_drop`), but this lets callers
    /// await the exit explicitly.
    ///
    /// # Errors
    ///
    /// Returns [`SdkError::Spawn`] when the kill syscall fails.
    pub async fn shutdown(mut self) -> Result<(), SdkError> {
        self.child
            .kill()
            .await
            .map_err(|e| SdkError::Spawn(e.to_string()))
    }
}

/// gRPC interceptor that injects the CSRF token header on every request.
#[derive(Clone)]
pub struct CsrfInterceptor {
    csrf: String,
}

impl tonic::service::Interceptor for CsrfInterceptor {
    fn call(
        &mut self,
        mut request: tonic::Request<()>,
    ) -> Result<tonic::Request<()>, tonic::Status> {
        match self.csrf.parse() {
            Ok(value) => {
                request.metadata_mut().insert(CSRF_HEADER, value);
                Ok(request)
            }
            Err(_) => Err(tonic::Status::internal("invalid CSRF token for metadata")),
        }
    }
}

/// Read lines from the language server stdout until the port line matches.
async fn read_port(stdout: tokio::process::ChildStdout) -> Result<u16, SdkError> {
    // `listening on <proto> port at (\d+) for HTTP(S)?` — case-insensitive.
    let re = regex::Regex::new(r"(?i)listening on \w+ port at (\d+) for HTTPS?")
        .expect("static regex is valid");

    let mut lines = BufReader::new(stdout).lines();
    while let Some(line) = lines
        .next_line()
        .await
        .map_err(|e| SdkError::Spawn(e.to_string()))?
    {
        debug!(target: "antigravity_sdk::local_ls", "ls: {line}");
        if let Some(caps) = re.captures(&line)
            && let Some(m) = caps.get(1)
            && let Ok(port) = m.as_str().parse::<u16>()
        {
            return Ok(port);
        }
    }
    Err(SdkError::PortTimeout)
}

/// Build a tonic [`Channel`] to `https://127.0.0.1:{port}` whose TLS layer pins
/// the language server's self-signed certificate by SPKI SHA-256.
async fn build_pinned_channel(port: u16) -> Result<Channel, SdkError> {
    let verifier = Arc::new(SpkiPinVerifier::new()?);

    let provider = CryptoProvider::get_default()
        .cloned()
        .unwrap_or_else(|| Arc::new(rustls::crypto::ring::default_provider()));

    let tls = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| SdkError::Tls(e.to_string()))?
        .dangerous()
        .with_custom_certificate_verifier(verifier)
        .with_no_client_auth();

    let https = hyper_rustls::HttpsConnectorBuilder::new()
        .with_tls_config(tls)
        .https_only()
        .enable_http2()
        .build();

    let uri = format!("https://127.0.0.1:{port}");
    let endpoint = Endpoint::from_shared(uri).map_err(|e| SdkError::Transport(e.to_string()))?;

    endpoint
        .connect_with_connector(https)
        .await
        .map_err(|e| SdkError::Transport(e.to_string()))
}

/// rustls server-certificate verifier that pins the end-entity certificate's
/// `SubjectPublicKeyInfo` SHA-256 against [`PINNED_SPKI_SHA256_B64`].
///
/// All other chain checks are intentionally bypassed because the endpoint is a
/// local, self-signed `127.0.0.1` listener; the SPKI pin is the trust anchor.
#[derive(Debug)]
struct SpkiPinVerifier {
    /// Decoded 32-byte SHA-256 of the pinned SPKI.
    pinned: Vec<u8>,
    /// Signature schemes supported by the active crypto provider.
    provider: Arc<CryptoProvider>,
}

impl SpkiPinVerifier {
    fn new() -> Result<Self, SdkError> {
        use base64::Engine as _;
        let pinned = base64::engine::general_purpose::STANDARD
            .decode(PINNED_SPKI_SHA256_B64)
            .map_err(|e| SdkError::Tls(format!("invalid pinned SPKI base64: {e}")))?;
        let provider = CryptoProvider::get_default()
            .cloned()
            .unwrap_or_else(|| Arc::new(rustls::crypto::ring::default_provider()));
        Ok(Self { pinned, provider })
    }
}

impl ServerCertVerifier for SpkiPinVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, RustlsError> {
        let (_, cert) = x509_parser::parse_x509_certificate(end_entity.as_ref()).map_err(|_| {
            RustlsError::InvalidCertificate(rustls::CertificateError::BadEncoding)
        })?;

        let spki_der = cert.tbs_certificate.subject_pki.raw;
        let digest = Sha256::digest(spki_der);

        // Constant-time-ish length + content check.
        if digest.as_slice() == self.pinned.as_slice() {
            Ok(ServerCertVerified::assertion())
        } else {
            Err(RustlsError::InvalidCertificate(
                rustls::CertificateError::Other(rustls::OtherError(Arc::new(SpkiPinMismatch))),
            ))
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        verify_tls12_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        verify_tls13_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

/// Error payload reported when the pinned SPKI does not match.
#[derive(Debug)]
struct SpkiPinMismatch;

impl std::fmt::Display for SpkiPinMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("language_server certificate SPKI does not match the pinned fingerprint")
    }
}

impl std::error::Error for SpkiPinMismatch {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinned_spki_decodes_to_32_bytes() {
        use base64::Engine as _;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(PINNED_SPKI_SHA256_B64)
            .expect("pin is valid base64");
        assert_eq!(decoded.len(), 32, "SHA-256 SPKI pin must be 32 bytes");
    }

    #[test]
    fn csrf_token_is_64_hex_chars() {
        let token = generate_csrf_token();
        assert_eq!(token.len(), 64);
        assert!(token.bytes().all(|b| b.is_ascii_hexdigit()));
    }

    #[test]
    fn launch_config_defaults() {
        let cfg = LaunchConfig::new("aphrody", "1.0.0", "/tmp/agy");
        assert_eq!(cfg.api_server_url, DEFAULT_API_SERVER_URL);
        assert_eq!(cfg.cloud_code_endpoint, DEFAULT_CLOUD_CODE_ENDPOINT);
        assert_eq!(cfg.https_server_port, 0);
        assert!(cfg.headless);
        assert!(cfg.csrf_token.is_none());
        // A generated CSRF token is produced on demand.
        assert_eq!(cfg.resolved_csrf().len(), 64);
    }

    #[test]
    fn port_regex_extracts_https_port() {
        let re = regex::Regex::new(r"(?i)listening on \w+ port at (\d+) for HTTPS?").unwrap();
        let caps = re
            .captures("Listening on local port at 51234 for HTTPS")
            .expect("matches");
        assert_eq!(&caps[1], "51234");

        let caps2 = re
            .captures("listening on TCP port at 8080 for HTTP")
            .expect("matches plain HTTP too");
        assert_eq!(&caps2[1], "8080");
    }
}

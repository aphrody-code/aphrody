// SPDX-License-Identifier: Apache-2.0
//! Native asynchronous SMTP client (RFC 5321 + 5322).
//!
//! Implements the minimum surface needed to deliver a single message:
//! EHLO → optional STARTTLS → AUTH PLAIN → MAIL FROM → RCPT TO → DATA → QUIT.
//!
//! TLS uses `tokio-rustls` with `webpki-roots` for trust. SMTP requires raw
//! sockets so the entire module is gated `cfg(not(target_arch = "wasm32"))`.

#![cfg(not(target_arch = "wasm32"))]

use std::sync::Arc;

use async_trait::async_trait;
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufStream};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use uuid::Uuid;

use crate::connector::{Connector, Message, MessageError, MessageId, MessageResult};

/// Transport security mode used when connecting to the SMTP server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TlsMode {
    /// Plaintext upgrade via STARTTLS (port 587 / 25).
    Starttls,
    /// Wrap the socket in TLS immediately (port 465).
    ImplicitTls,
    /// No TLS at all — local relays / tests only.
    Plain,
}

/// SMTP connector configuration.
#[derive(Debug, Clone)]
pub struct SmtpConnector {
    host: String,
    port: u16,
    username: String,
    password: String,
    tls_mode: TlsMode,
    default_from: Option<String>,
    /// Hostname advertised in EHLO + Message-ID right-hand side.
    local_hostname: String,
}

impl SmtpConnector {
    /// Build a connector with all fields.
    pub fn new(
        host: impl Into<String>,
        port: u16,
        username: impl Into<String>,
        password: impl Into<String>,
        tls_mode: TlsMode,
    ) -> Self {
        let local = gethostname::gethostname().to_string_lossy().into_owned();
        let local_hostname = if local.is_empty() { "localhost".to_owned() } else { local };
        Self {
            host: host.into(),
            port,
            username: username.into(),
            password: password.into(),
            tls_mode,
            default_from: None,
            local_hostname,
        }
    }

    /// Override the `From:` address used when a [`Message`] doesn't carry one.
    pub fn with_default_from(mut self, from: impl Into<String>) -> Self {
        self.default_from = Some(from.into());
        self
    }

    /// Override the EHLO + Message-ID hostname (defaults to OS hostname).
    pub fn with_local_hostname(mut self, host: impl Into<String>) -> Self {
        self.local_hostname = host.into();
        self
    }

    /// Generate an RFC 5322-compliant Message-ID.
    ///
    /// Format: `<UUIDv4@hostname>` — globally unique, no PII leak beyond
    /// the hostname (which is bounded by `with_local_hostname`).
    pub fn make_message_id(&self) -> String {
        format!("<{}@{}>", Uuid::new_v4(), self.local_hostname)
    }

    /// Render a full RFC 5322 message body from a [`Message`] + chosen Message-ID.
    pub(crate) fn render_rfc5322(&self, msg: &Message, message_id: &str, from: &str) -> String {
        let date = chrono::Utc::now().format("%a, %d %b %Y %H:%M:%S +0000").to_string();
        let subject = msg.subject.as_deref().unwrap_or("(no subject)");
        let mut out = String::with_capacity(msg.body.len() + 256);
        out.push_str(&format!("From: {from}\r\n"));
        out.push_str(&format!("To: {}\r\n", msg.to));
        out.push_str(&format!("Subject: {subject}\r\n"));
        out.push_str(&format!("Date: {date}\r\n"));
        out.push_str(&format!("Message-ID: {message_id}\r\n"));
        out.push_str("MIME-Version: 1.0\r\n");
        out.push_str("Content-Type: text/plain; charset=utf-8\r\n");
        out.push_str("Content-Transfer-Encoding: 8bit\r\n");
        out.push_str("\r\n");
        // RFC 5321 §4.5.2 dot-stuffing.
        for line in msg.body.split('\n') {
            let line = line.strip_suffix('\r').unwrap_or(line);
            if line.starts_with('.') {
                out.push('.');
            }
            out.push_str(line);
            out.push_str("\r\n");
        }
        out
    }
}

// ── Wire helpers ──────────────────────────────────────────────────────────────

async fn read_reply<R>(stream: &mut R) -> MessageResult<(u16, String)>
where
    R: AsyncBufReadExt + Unpin,
{
    let mut full = String::new();
    loop {
        let mut line = String::new();
        let n = stream.read_line(&mut line).await?;
        if n == 0 {
            return Err(MessageError::Smtp { code: 0, message: "connection closed".to_owned() });
        }
        full.push_str(&line);
        // SMTP multi-line: "NNN-text" continues, "NNN text" terminates.
        if line.len() >= 4 {
            let bytes = line.as_bytes();
            if bytes[3] == b' ' {
                break;
            }
        } else {
            break;
        }
    }
    let code = full.get(..3).and_then(|s| s.parse::<u16>().ok()).ok_or_else(|| {
        MessageError::Smtp { code: 0, message: format!("malformed reply: {full}") }
    })?;
    Ok((code, full))
}

async fn write_line<W>(stream: &mut W, line: &str) -> MessageResult<()>
where
    W: AsyncWriteExt + Unpin,
{
    stream.write_all(line.as_bytes()).await?;
    stream.write_all(b"\r\n").await?;
    stream.flush().await?;
    Ok(())
}

async fn expect_reply<R>(stream: &mut R, expected: u16) -> MessageResult<String>
where
    R: AsyncBufReadExt + Unpin,
{
    let (code, full) = read_reply(stream).await?;
    if code != expected {
        return Err(MessageError::Smtp { code, message: full });
    }
    Ok(full)
}

fn build_tls_connector() -> MessageResult<TlsConnector> {
    let mut roots = rustls::RootCertStore::empty();
    roots.roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let cfg = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    Ok(TlsConnector::from(Arc::new(cfg)))
}

/// Run the EHLO → AUTH → MAIL → RCPT → DATA → QUIT dialog on `stream`.
/// `stream` must already have read the 220 greeting on caller's behalf? — no,
/// this function expects an untouched, freshly-connected stream.
async fn run_smtp_dialog<S>(
    stream: S,
    cfg: &SmtpConnector,
    msg: &Message,
    message_id: &str,
    from: &str,
) -> MessageResult<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut buf = BufStream::new(stream);
    expect_reply(&mut buf, 220).await?;
    write_line(&mut buf, &format!("EHLO {}", cfg.local_hostname)).await?;
    expect_reply(&mut buf, 250).await?;

    if !cfg.username.is_empty() {
        let credential = format!("\0{}\0{}", cfg.username, cfg.password);
        let encoded = base64::engine::general_purpose::STANDARD.encode(credential.as_bytes());
        write_line(&mut buf, &format!("AUTH PLAIN {encoded}")).await?;
        expect_reply(&mut buf, 235).await?;
    }

    write_line(&mut buf, &format!("MAIL FROM:<{from}>")).await?;
    expect_reply(&mut buf, 250).await?;

    write_line(&mut buf, &format!("RCPT TO:<{}>", msg.to)).await?;
    expect_reply(&mut buf, 250).await?;

    write_line(&mut buf, "DATA").await?;
    expect_reply(&mut buf, 354).await?;

    let body = cfg.render_rfc5322(msg, message_id, from);
    buf.write_all(body.as_bytes()).await?;
    write_line(&mut buf, ".").await?;
    expect_reply(&mut buf, 250).await?;

    write_line(&mut buf, "QUIT").await?;
    Ok(())
}

/// STARTTLS dialog: EHLO → STARTTLS → upgrade → EHLO → AUTH → MAIL → ...
async fn run_starttls_dialog(
    tcp: TcpStream,
    cfg: &SmtpConnector,
    msg: &Message,
    message_id: &str,
    from: &str,
) -> MessageResult<()> {
    let mut buf = BufStream::new(tcp);
    expect_reply(&mut buf, 220).await?;
    write_line(&mut buf, &format!("EHLO {}", cfg.local_hostname)).await?;
    let ehlo = expect_reply(&mut buf, 250).await?;
    if !ehlo.to_ascii_uppercase().contains("STARTTLS") {
        return Err(MessageError::Smtp {
            code: 0,
            message: "server does not advertise STARTTLS".to_owned(),
        });
    }
    write_line(&mut buf, "STARTTLS").await?;
    expect_reply(&mut buf, 220).await?;
    buf.flush().await?;
    let inner = buf.into_inner();

    let connector = build_tls_connector()?;
    let server_name = rustls::pki_types::ServerName::try_from(cfg.host.clone())
        .map_err(|e| MessageError::Unexpected(format!("invalid TLS server name: {e}")))?;
    let tls = connector
        .connect(server_name, inner)
        .await
        .map_err(|e| MessageError::Http(format!("STARTTLS handshake failed: {e}")))?;

    // Re-issue EHLO on encrypted channel, then proceed normally — but without
    // the initial 220 greeting (already consumed).
    let mut buf = BufStream::new(tls);
    write_line(&mut buf, &format!("EHLO {}", cfg.local_hostname)).await?;
    expect_reply(&mut buf, 250).await?;

    if !cfg.username.is_empty() {
        let credential = format!("\0{}\0{}", cfg.username, cfg.password);
        let encoded = base64::engine::general_purpose::STANDARD.encode(credential.as_bytes());
        write_line(&mut buf, &format!("AUTH PLAIN {encoded}")).await?;
        expect_reply(&mut buf, 235).await?;
    }

    write_line(&mut buf, &format!("MAIL FROM:<{from}>")).await?;
    expect_reply(&mut buf, 250).await?;
    write_line(&mut buf, &format!("RCPT TO:<{}>", msg.to)).await?;
    expect_reply(&mut buf, 250).await?;
    write_line(&mut buf, "DATA").await?;
    expect_reply(&mut buf, 354).await?;

    let body = cfg.render_rfc5322(msg, message_id, from);
    buf.write_all(body.as_bytes()).await?;
    write_line(&mut buf, ".").await?;
    expect_reply(&mut buf, 250).await?;
    write_line(&mut buf, "QUIT").await?;
    Ok(())
}

#[async_trait]
impl Connector for SmtpConnector {
    fn id(&self) -> &'static str {
        "smtp"
    }

    async fn send(&self, msg: Message) -> MessageResult<MessageId> {
        let from_addr = msg
            .from
            .clone()
            .or_else(|| self.default_from.clone())
            .unwrap_or_else(|| self.username.clone());
        if from_addr.is_empty() {
            return Err(MessageError::MissingConfig("from / default_from / username"));
        }
        let message_id = self.make_message_id();

        let tcp = TcpStream::connect((self.host.as_str(), self.port)).await?;
        match self.tls_mode {
            TlsMode::Plain => {
                run_smtp_dialog(tcp, self, &msg, &message_id, &from_addr).await?;
            }
            TlsMode::ImplicitTls => {
                let connector = build_tls_connector()?;
                let server_name = rustls::pki_types::ServerName::try_from(self.host.clone())
                    .map_err(|e| MessageError::Unexpected(format!("invalid TLS server name: {e}")))?;
                let tls = connector.connect(server_name, tcp).await.map_err(|e| {
                    MessageError::Http(format!("TLS handshake failed: {e}"))
                })?;
                run_smtp_dialog(tls, self, &msg, &message_id, &from_addr).await?;
            }
            TlsMode::Starttls => {
                run_starttls_dialog(tcp, self, &msg, &message_id, &from_addr).await?;
            }
        }

        Ok(MessageId(message_id))
    }

    async fn health(&self) -> MessageResult<()> {
        let tcp = TcpStream::connect((self.host.as_str(), self.port)).await?;
        let mut buf = BufStream::new(tcp);
        expect_reply(&mut buf, 220).await?;
        write_line(&mut buf, &format!("EHLO {}", self.local_hostname)).await?;
        expect_reply(&mut buf, 250).await?;
        write_line(&mut buf, "QUIT").await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smtp_connector_id_is_smtp() {
        let c = SmtpConnector::new("smtp.example.com", 587, "u", "p", TlsMode::Starttls);
        assert_eq!(c.id(), "smtp");
    }

    #[test]
    fn message_id_format_is_rfc5322() {
        let c = SmtpConnector::new("h", 587, "u", "p", TlsMode::Plain)
            .with_local_hostname("aphrody.local");
        let id = c.make_message_id();
        assert!(id.starts_with('<'), "expected leading <: {id}");
        assert!(id.ends_with('>'), "expected trailing >: {id}");
        assert!(id.contains('@'), "expected @: {id}");
        assert!(id.contains("aphrody.local"), "expected hostname in id: {id}");
        let inner = &id[1..id.len() - 1];
        let (left, _right) = inner.split_once('@').expect("must split on @");
        assert_eq!(left.len(), 36, "UUID v4 length");
    }

    #[test]
    fn render_rfc5322_dot_stuffs() {
        let c = SmtpConnector::new("h", 25, "u", "p", TlsMode::Plain)
            .with_local_hostname("h");
        let msg = Message::text("to@x", ".dotted\nnormal\n.last");
        let rendered = c.render_rfc5322(&msg, "<id@h>", "from@x");
        assert!(rendered.contains("\r\n..dotted\r\n"), "got: {rendered}");
        assert!(rendered.contains("..last\r\n"), "got: {rendered}");
    }

    #[test]
    fn render_rfc5322_contains_headers() {
        let c = SmtpConnector::new("h", 25, "u", "p", TlsMode::Plain)
            .with_local_hostname("h");
        let msg = Message::text("to@x", "hello").with_subject("Hi");
        let r = c.render_rfc5322(&msg, "<id@h>", "from@x");
        assert!(r.contains("From: from@x\r\n"));
        assert!(r.contains("To: to@x\r\n"));
        assert!(r.contains("Subject: Hi\r\n"));
        assert!(r.contains("Message-ID: <id@h>\r\n"));
        assert!(r.contains("MIME-Version: 1.0\r\n"));
        assert!(r.ends_with("hello\r\n"));
    }
}

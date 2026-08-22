// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors
//
// A minimal HTTP/1.1 client over a plain TCP socket.
//
// This exists because `reqwest::blocking` cannot be used here. It builds a
// tokio runtime internally, and the CLI already runs inside one; every request
// then stalls with no error and no timeout — the failure looks exactly like a
// hung model. Moving the loop to its own OS thread was not enough, because the
// blocking client reaches for a shared runtime rather than a thread-local one.
//
// What is actually needed is small: POST a JSON body to loopback, read a JSON
// body back. No TLS (the server is on 127.0.0.1), no redirects, no
// compression, no cookies, no connection reuse. That is sixty lines of the
// 1997 spec, and it is deterministic — which a nested runtime is not.
//
// Deliberately NOT a general HTTP client: it speaks only to `llama-server` on
// loopback and rejects anything that is not a well-formed response.

use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::net::TcpStream;
use std::time::Duration;

use crate::{OcrError, Result};

/// A response from the local server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    /// HTTP status code.
    pub status: u16,
    /// Response body, decoded as UTF-8 (lossily — a model can emit anything).
    pub body: String,
}

impl Response {
    /// Whether the status is 2xx.
    #[must_use]
    pub const fn is_success(&self) -> bool {
        self.status >= 200 && self.status < 300
    }
}

/// GET a path on `127.0.0.1:<port>`.
///
/// # Errors
///
/// [`OcrError::Process`] on connect, write, read or parse failure.
pub fn get(port: u16, path: &str, timeout: Duration) -> Result<Response> {
    let request =
        format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n");
    exchange(port, request.as_bytes(), timeout)
}

/// POST a JSON body to a path on `127.0.0.1:<port>`.
///
/// # Errors
///
/// [`OcrError::Process`] on connect, write, read or parse failure.
pub fn post_json(port: u16, path: &str, body: &str, timeout: Duration) -> Result<Response> {
    let mut request = format!(
        "POST {path} HTTP/1.1\r\n\
         Host: 127.0.0.1:{port}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    request.extend_from_slice(body.as_bytes());
    exchange(port, &request, timeout)
}

/// Send a request and read the whole response.
fn exchange(port: u16, request: &[u8], timeout: Duration) -> Result<Response> {
    let address = format!("127.0.0.1:{port}");
    let stream = TcpStream::connect(&address).map_err(|e| fail(&address, "connect", &e))?;
    // Both directions get a deadline: a server that accepts the connection and
    // then goes quiet must not hang a ten-thousand-page batch.
    stream.set_read_timeout(Some(timeout)).map_err(|e| fail(&address, "set read timeout", &e))?;
    stream.set_write_timeout(Some(timeout)).map_err(|e| fail(&address, "set write timeout", &e))?;

    let mut stream = stream;
    stream.write_all(request).map_err(|e| fail(&address, "write", &e))?;
    stream.flush().map_err(|e| fail(&address, "flush", &e))?;

    let mut reader = BufReader::new(stream);
    let status = read_status_line(&mut reader, &address)?;
    let headers = read_headers(&mut reader, &address)?;
    let body = read_body(&mut reader, &headers, &address)?;

    Ok(Response { status, body })
}

/// Parse `HTTP/1.1 200 OK`.
fn read_status_line(reader: &mut BufReader<TcpStream>, address: &str) -> Result<u16> {
    let mut line = String::new();
    reader.read_line(&mut line).map_err(|e| fail(address, "read status line", &e))?;
    let code = line
        .split_whitespace()
        .nth(1)
        .and_then(|c| c.parse::<u16>().ok())
        .ok_or_else(|| OcrError::Process {
            command: address.to_owned(),
            status: "malformed status line".to_owned(),
            stderr: reason_tail(&line),
        })?;
    Ok(code)
}

/// Collect headers, lower-cased, until the blank line.
fn read_headers(
    reader: &mut BufReader<TcpStream>,
    address: &str,
) -> Result<std::collections::BTreeMap<String, String>> {
    let mut headers = std::collections::BTreeMap::new();
    loop {
        let mut line = String::new();
        let read = reader.read_line(&mut line).map_err(|e| fail(address, "read header", &e))?;
        // A connection that closes mid-headers is a failure, not an end.
        if read == 0 {
            return Err(OcrError::Process {
                command: address.to_owned(),
                status: "connection closed in headers".to_owned(),
                stderr: String::new(),
            });
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            return Ok(headers);
        }
        if let Some((name, value)) = trimmed.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_owned());
        }
    }
}

/// Read the body, honouring `Content-Length` or chunked transfer.
fn read_body(
    reader: &mut BufReader<TcpStream>,
    headers: &std::collections::BTreeMap<String, String>,
    address: &str,
) -> Result<String> {
    if headers.get("transfer-encoding").is_some_and(|v| v.eq_ignore_ascii_case("chunked")) {
        return read_chunked(reader, address);
    }

    let mut buffer = Vec::new();
    if let Some(length) = headers.get("content-length").and_then(|v| v.parse::<usize>().ok()) {
        buffer.resize(length, 0);
        reader.read_exact(&mut buffer).map_err(|e| fail(address, "read body", &e))?;
    } else {
        // `Connection: close` with no length: read to EOF.
        reader.read_to_end(&mut buffer).map_err(|e| fail(address, "read body", &e))?;
    }
    Ok(String::from_utf8_lossy(&buffer).into_owned())
}

/// Read a chunked body: size line in hex, then that many bytes, until a zero.
fn read_chunked(reader: &mut BufReader<TcpStream>, address: &str) -> Result<String> {
    let mut body = Vec::new();
    loop {
        let mut size_line = String::new();
        reader.read_line(&mut size_line).map_err(|e| fail(address, "read chunk size", &e))?;
        let size = usize::from_str_radix(size_line.trim(), 16).map_err(|_| OcrError::Process {
            command: address.to_owned(),
            status: "malformed chunk size".to_owned(),
            stderr: reason_tail(&size_line),
        })?;
        if size == 0 {
            return Ok(String::from_utf8_lossy(&body).into_owned());
        }
        let mut chunk = vec![0_u8; size];
        reader.read_exact(&mut chunk).map_err(|e| fail(address, "read chunk", &e))?;
        body.extend_from_slice(&chunk);
        // Consume the CRLF that terminates the chunk.
        let mut crlf = String::new();
        reader.read_line(&mut crlf).map_err(|e| fail(address, "read chunk terminator", &e))?;
    }
}

/// Build a transport failure.
fn fail(address: &str, what: &str, error: &std::io::Error) -> OcrError {
    OcrError::Process {
        command: address.to_owned(),
        status: what.to_owned(),
        stderr: error.to_string(),
    }
}

/// A bounded sample of a malformed line, for an error message.
fn reason_tail(line: &str) -> String {
    line.trim().chars().take(120).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::net::TcpListener;

    /// Serve one canned response on a free loopback port.
    fn serve_once(response: &'static [u8]) -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let port = listener.local_addr().expect("local addr").port();
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                // Drain the request so the client's write completes.
                let mut buffer = [0_u8; 4096];
                let _ = std::io::Read::read(&mut stream, &mut buffer);
                let _ = stream.write_all(response);
                let _ = stream.flush();
            }
        });
        port
    }

    #[test]
    fn a_content_length_response_is_read_whole() {
        let port = serve_once(
            b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 17\r\n\r\n{\"status\":\"ok\"}\r\n",
        );
        let response = get(port, "/health", Duration::from_secs(5)).unwrap();
        assert_eq!(response.status, 200);
        assert!(response.is_success());
        assert!(response.body.starts_with("{\"status\":\"ok\"}"), "{}", response.body);
    }

    #[test]
    fn a_chunked_response_is_reassembled() {
        // llama-server answers chunked when it does not know the length up
        // front, which is exactly the streaming-adjacent case.
        let port = serve_once(
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n6\r\n world\r\n0\r\n\r\n",
        );
        let response = get(port, "/x", Duration::from_secs(5)).unwrap();
        assert_eq!(response.body, "hello world");
    }

    #[test]
    fn a_post_sends_its_body_and_reads_the_answer() {
        let port = serve_once(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\ndone");
        let response =
            post_json(port, "/v1/chat/completions", "{\"a\":1}", Duration::from_secs(5)).unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(response.body, "done");
    }

    #[test]
    fn a_non_2xx_status_is_reported_not_hidden() {
        let port = serve_once(b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 5\r\n\r\nboom!");
        let response = get(port, "/x", Duration::from_secs(5)).unwrap();
        assert_eq!(response.status, 500);
        assert!(!response.is_success());
        assert_eq!(response.body, "boom!");
    }

    #[test]
    fn a_closed_port_fails_fast_instead_of_hanging() {
        // Bind then drop, so the port is almost certainly free.
        let port = {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.local_addr().unwrap().port()
        };
        let error = get(port, "/health", Duration::from_secs(2)).unwrap_err();
        assert!(error.to_string().contains("connect"), "{error}");
    }

    #[test]
    fn a_malformed_status_line_is_an_error() {
        let port = serve_once(b"not http at all\r\n\r\n");
        assert!(get(port, "/x", Duration::from_secs(5)).is_err());
    }

    #[test]
    fn a_body_with_no_length_is_read_to_close() {
        let port = serve_once(b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\nbare body");
        assert_eq!(get(port, "/x", Duration::from_secs(5)).unwrap().body, "bare body");
    }

    #[test]
    fn success_is_the_2xx_range_only() {
        for (status, expected) in [(199, false), (200, true), (204, true), (299, true), (300, false), (404, false)] {
            let response = Response { status, body: String::new() };
            assert_eq!(response.is_success(), expected, "{status}");
        }
    }
}

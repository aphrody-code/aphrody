// SPDX-License-Identifier: Apache-2.0
//! SSRF (Server-Side Request Forgery) guard for BYOK upstream URLs.
//!
//! Mirrors the contracts-level policy enforced by the upstream daemon:
//! reject any URL whose hostname (literal IP) or resolved address falls
//! within a private, link-local, multicast, broadcast, loopback, or
//! IPv4-mapped-private range, plus a small set of scheme/file shortcuts.
//!
//! Two entry points are provided:
//!
//! * [`check_url_ssrf`] runs the synchronous literal-hostname check used at
//!   parse time.  It is the cheapest guard and covers the obvious cases
//!   (`http://127.0.0.1`, `http://10.0.0.5`, `file:///etc/passwd`).
//! * [`check_url_ssrf_resolved`] additionally performs DNS resolution and
//!   re-runs the block-list against every address the system would actually
//!   connect to, defeating DNS rebinding / public-name-pointing-at-internal
//!   tricks (`internal.example.com → 10.0.0.5`).
//!
//! The block ranges intentionally match the upstream TypeScript implementation
//! at `packages/contracts/src/api/connectionTest.ts`:
//!
//! | IPv4 range            | Notes                              |
//! |-----------------------|------------------------------------|
//! | `0.0.0.0/8`           | "this network" / unspecified       |
//! | `10.0.0.0/8`          | RFC 1918 private                   |
//! | `100.64.0.0/10`       | Carrier-grade NAT (RFC 6598)       |
//! | `127.0.0.0/8`         | Loopback                           |
//! | `169.254.0.0/16`      | Link-local (RFC 3927)              |
//! | `172.16.0.0/12`       | RFC 1918 private                   |
//! | `192.168.0.0/16`      | RFC 1918 private                   |
//! | `224.0.0.0/4`         | Multicast + reserved (≥ 224.x.x.x) |
//!
//! | IPv6 range            | Notes                              |
//! |-----------------------|------------------------------------|
//! | `::1/128`             | Loopback                           |
//! | `::/128`              | Unspecified                        |
//! | `fc00::/7`            | Unique-local                       |
//! | `fe80::/10`           | Link-local                         |
//! | `::ffff:0:0/96`       | IPv4-mapped (recurses into IPv4)   |

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use url::Url;

use crate::GatewayError;

// ---------------------------------------------------------------------------
// Public helpers
// ---------------------------------------------------------------------------

/// Synchronous SSRF guard.
///
/// Rejects when:
///
/// * the URL scheme is not `http` or `https`;
/// * the hostname is missing;
/// * the hostname is `localhost` (case-insensitive, with optional FQDN trailing dot);
/// * the hostname parses to an IP literal in any of the blocked ranges
///   listed in the module documentation.
///
/// Hostnames that do *not* parse as an IP literal are passed through; pair
/// this call with [`check_url_ssrf_resolved`] when an async DNS lookup is
/// available to also catch public DNS names pointing at internal addresses.
pub fn check_url_ssrf(url: &Url) -> Result<(), GatewayError> {
    let scheme = url.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(GatewayError::Config(format!("SSRF: scheme `{scheme}` not allowed (only http/https)")));
    }

    let host = url
        .host_str()
        .ok_or_else(|| GatewayError::Config("SSRF: URL has no host".to_owned()))?;
    let normalized = normalize_hostname(host);

    if normalized == "localhost" {
        return Err(GatewayError::Config("SSRF: localhost not allowed".to_owned()));
    }

    if let Some(ip) = parse_ip_literal(&normalized) {
        if is_blocked_ip(ip) {
            return Err(GatewayError::Config(format!("SSRF: blocked IP literal `{ip}`")));
        }
    }

    Ok(())
}

/// Asynchronous, DNS-aware SSRF guard.
///
/// Runs [`check_url_ssrf`] first.  When the hostname is not already an IP
/// literal, resolves it via the system resolver (`tokio::net::lookup_host`)
/// and re-applies the block list to every returned address — defeating DNS
/// rebinding and public-name-pointing-at-internal tricks
/// (`internal.example.com -> 10.0.0.5`).
///
/// DNS failures are surfaced as a config error so the caller can distinguish
/// a deliberately blocked target ("resolved to 10.0.0.1") from a transient
/// resolver hiccup ("name not found") in their UX.
pub async fn check_url_ssrf_resolved(url: &Url) -> Result<(), GatewayError> {
    check_url_ssrf(url)?;
    let host = url.host_str().unwrap_or("");
    let normalized = normalize_hostname(host);
    if parse_ip_literal(&normalized).is_some() {
        // Already validated by the literal check above.
        return Ok(());
    }
    let port = url.port_or_known_default().unwrap_or(443);
    let target = format!("{normalized}:{port}");
    let addrs = tokio::net::lookup_host(target).await.map_err(|e| {
        GatewayError::Config(format!("SSRF: DNS lookup for `{normalized}` failed: {e}"))
    })?;
    for addr in addrs {
        if is_blocked_ip(addr.ip()) {
            return Err(GatewayError::Config(format!(
                "SSRF: `{normalized}` resolves to blocked address `{}`",
                addr.ip()
            )));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Hostname / IP parsing
// ---------------------------------------------------------------------------

/// Lowercases the hostname and strips one or more trailing dots (FQDN form),
/// matching the upstream `normalizeBracketedIpv6` helper.
pub(crate) fn normalize_hostname(host: &str) -> String {
    let stripped = if host.starts_with('[') && host.ends_with(']') {
        &host[1..host.len() - 1]
    } else {
        host
    };
    let lower = stripped.to_lowercase();
    let trimmed = lower.trim_end_matches('.').to_owned();
    trimmed
}

/// Parses a literal IPv4 or IPv6 address.
///
/// Returns `None` for plain DNS names — those need to go through the async
/// resolver path to be blocked.
pub(crate) fn parse_ip_literal(host: &str) -> Option<IpAddr> {
    if let Ok(v4) = host.parse::<Ipv4Addr>() {
        return Some(IpAddr::V4(v4));
    }
    if let Ok(v6) = host.parse::<Ipv6Addr>() {
        return Some(IpAddr::V6(v6));
    }
    None
}

// ---------------------------------------------------------------------------
// Block-list evaluation
// ---------------------------------------------------------------------------

/// Returns `true` when `ip` falls within any blocked range.
pub fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_blocked_ipv4(v4),
        IpAddr::V6(v6) => is_blocked_ipv6(v6),
    }
}

fn is_blocked_ipv4(ip: Ipv4Addr) -> bool {
    let [a, b, _, _] = ip.octets();
    if a == 0 {
        return true; // 0.0.0.0/8
    }
    if a == 10 {
        return true; // 10.0.0.0/8 — RFC 1918
    }
    if a == 100 && (64..=127).contains(&b) {
        return true; // 100.64.0.0/10 — carrier-grade NAT
    }
    if a == 127 {
        return true; // 127.0.0.0/8 — loopback
    }
    if a == 169 && b == 254 {
        return true; // 169.254.0.0/16 — link-local
    }
    if a == 172 && (16..=31).contains(&b) {
        return true; // 172.16.0.0/12 — RFC 1918
    }
    if a == 192 && b == 168 {
        return true; // 192.168.0.0/16 — RFC 1918
    }
    if a >= 224 {
        return true; // 224.0.0.0/4 multicast + 240.0.0.0/4 reserved + 255.255.255.255 broadcast
    }
    false
}

fn is_blocked_ipv6(ip: Ipv6Addr) -> bool {
    if ip == Ipv6Addr::LOCALHOST {
        return true; // ::1
    }
    if ip == Ipv6Addr::UNSPECIFIED {
        return true; // ::
    }
    let segs = ip.segments();
    // fc00::/7 — unique-local
    if (segs[0] & 0xfe00) == 0xfc00 {
        return true;
    }
    // fe80::/10 — link-local
    if (segs[0] & 0xffc0) == 0xfe80 {
        return true;
    }
    // ::ffff:0:0/96 — IPv4-mapped
    if segs[0] == 0
        && segs[1] == 0
        && segs[2] == 0
        && segs[3] == 0
        && segs[4] == 0
        && segs[5] == 0xffff
    {
        let mapped = Ipv4Addr::new(
            (segs[6] >> 8) as u8,
            (segs[6] & 0xff) as u8,
            (segs[7] >> 8) as u8,
            (segs[7] & 0xff) as u8,
        );
        return is_blocked_ipv4(mapped);
    }
    false
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> Url {
        Url::parse(s).expect("parse url")
    }

    // --- Scheme rejection ---------------------------------------------------

    #[test]
    fn ssrf_reject_file_scheme() {
        let err = check_url_ssrf(&url("file:///etc/passwd")).unwrap_err();
        assert!(matches!(err, GatewayError::Config(ref m) if m.contains("scheme")), "got: {err:?}");
    }

    #[test]
    fn ssrf_reject_gopher_scheme() {
        // gopher:// is parsed by url crate but not http/https → rejected.
        let err = check_url_ssrf(&url("gopher://example.com")).unwrap_err();
        assert!(matches!(err, GatewayError::Config(ref m) if m.contains("scheme")));
    }

    // --- localhost / IP literal rejection -----------------------------------

    #[test]
    fn ssrf_reject_localhost_literal() {
        let err = check_url_ssrf(&url("http://localhost:8080/v1/chat")).unwrap_err();
        assert!(matches!(err, GatewayError::Config(ref m) if m.contains("localhost")));
    }

    #[test]
    fn ssrf_reject_localhost_fqdn() {
        let err = check_url_ssrf(&url("http://localhost./v1/chat")).unwrap_err();
        assert!(matches!(err, GatewayError::Config(ref m) if m.contains("localhost")));
    }

    #[test]
    fn ssrf_reject_127_0_0_1() {
        let err = check_url_ssrf(&url("http://127.0.0.1/v1/chat")).unwrap_err();
        assert!(matches!(err, GatewayError::Config(ref m) if m.contains("blocked IP")));
    }

    #[test]
    fn ssrf_reject_127_255_255_254() {
        // Anywhere in 127.0.0.0/8.
        let err = check_url_ssrf(&url("http://127.255.255.254/")).unwrap_err();
        assert!(matches!(err, GatewayError::Config(_)));
    }

    #[test]
    fn ssrf_reject_10_0_0_5() {
        let err = check_url_ssrf(&url("http://10.0.0.5/v1/chat")).unwrap_err();
        assert!(matches!(err, GatewayError::Config(_)));
    }

    #[test]
    fn ssrf_reject_172_20_0_1() {
        let err = check_url_ssrf(&url("http://172.20.0.1/")).unwrap_err();
        assert!(matches!(err, GatewayError::Config(_)));
    }

    #[test]
    fn ssrf_reject_192_168_1_1() {
        let err = check_url_ssrf(&url("http://192.168.1.1/")).unwrap_err();
        assert!(matches!(err, GatewayError::Config(_)));
    }

    #[test]
    fn ssrf_reject_169_254_169_254() {
        // AWS / Azure / GCP instance metadata service IP.
        let err = check_url_ssrf(&url("http://169.254.169.254/latest/meta-data")).unwrap_err();
        assert!(matches!(err, GatewayError::Config(_)));
    }

    #[test]
    fn ssrf_reject_0_0_0_0() {
        let err = check_url_ssrf(&url("http://0.0.0.0/")).unwrap_err();
        assert!(matches!(err, GatewayError::Config(_)));
    }

    #[test]
    fn ssrf_reject_multicast_224() {
        let err = check_url_ssrf(&url("http://224.0.0.1/")).unwrap_err();
        assert!(matches!(err, GatewayError::Config(_)));
    }

    #[test]
    fn ssrf_reject_carrier_grade_nat_100_64() {
        let err = check_url_ssrf(&url("http://100.64.0.1/")).unwrap_err();
        assert!(matches!(err, GatewayError::Config(_)));
    }

    // --- IPv6 rejection -----------------------------------------------------

    #[test]
    fn ssrf_reject_ipv6_loopback() {
        let err = check_url_ssrf(&url("http://[::1]/v1/chat")).unwrap_err();
        assert!(matches!(err, GatewayError::Config(_)));
    }

    #[test]
    fn ssrf_reject_ipv6_unspecified() {
        let err = check_url_ssrf(&url("http://[::]/v1/chat")).unwrap_err();
        assert!(matches!(err, GatewayError::Config(_)));
    }

    #[test]
    fn ssrf_reject_ipv6_link_local() {
        let err = check_url_ssrf(&url("http://[fe80::1]/")).unwrap_err();
        assert!(matches!(err, GatewayError::Config(_)));
    }

    #[test]
    fn ssrf_reject_ipv6_unique_local() {
        let err = check_url_ssrf(&url("http://[fc00::1]/")).unwrap_err();
        assert!(matches!(err, GatewayError::Config(_)));
    }

    #[test]
    fn ssrf_reject_ipv4_mapped_ipv6_internal() {
        // ::ffff:10.0.0.1 maps to the blocked 10.0.0.1.
        let err = check_url_ssrf(&url("http://[::ffff:10.0.0.1]/")).unwrap_err();
        assert!(matches!(err, GatewayError::Config(_)));
    }

    // --- Allow list ---------------------------------------------------------

    #[test]
    fn ssrf_accept_public_ipv4_1_1_1_1() {
        check_url_ssrf(&url("https://1.1.1.1/")).expect("public IPv4 accepted");
    }

    #[test]
    fn ssrf_accept_public_dns_name() {
        check_url_ssrf(&url("https://api.openai.com/v1/chat/completions"))
            .expect("public DNS name accepted (literal check only)");
    }

    #[test]
    fn ssrf_accept_public_ipv6_google_dns() {
        check_url_ssrf(&url("https://[2001:4860:4860::8888]/")).expect("public IPv6 accepted");
    }

    // --- normalize_hostname unit ---------------------------------------------

    #[test]
    fn normalize_hostname_strips_brackets_and_dots() {
        assert_eq!(normalize_hostname("LOCALHOST."), "localhost");
        assert_eq!(normalize_hostname("[::1]"), "::1");
        assert_eq!(normalize_hostname("Example.COM."), "example.com");
    }

    // --- parse_ip_literal unit -----------------------------------------------

    #[test]
    fn parse_ip_literal_handles_both_families() {
        assert!(matches!(parse_ip_literal("127.0.0.1"), Some(IpAddr::V4(_))));
        assert!(matches!(parse_ip_literal("::1"), Some(IpAddr::V6(_))));
        assert!(parse_ip_literal("example.com").is_none());
    }
}

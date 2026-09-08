//! Small IP/host parsing helpers shared by blocklist, GeoIP, and engine.
//!
//! The previous `split(':')` approach truncated every IPv6 address at the
//! first colon (`2001:db8::1` became `"2001"`). These helpers are IPv6-safe.

use std::net::IpAddr;

/// Strip a trailing `:port` (or `[v6]:port`) from an endpoint string,
/// returning the host part. Bare IPs — v4 or v6 — are returned unchanged.
pub fn strip_port(raw: &str) -> &str {
    let s = raw.trim();
    // Bracketed form: [2001:db8::1]:443
    if let Some(rest) = s.strip_prefix('[') {
        if let Some(end) = rest.find(']') {
            return rest[..end].trim();
        }
        return s;
    }
    // A bare IP (v4 or v6) contains no port — return as-is.
    if s.parse::<IpAddr>().is_ok() {
        return s;
    }
    // Only strip when there is exactly one colon and a numeric port,
    // i.e. unambiguous `host:port`. Anything else is kept whole so
    // unbracketed IPv6 without port is never mangled.
    if s.chars().filter(|&c| c == ':').count() == 1
        && let Some((host, port)) = s.rsplit_once(':')
        && !host.is_empty()
        && !port.is_empty()
        && port.chars().all(|c| c.is_ascii_digit())
    {
        return host;
    }
    s
}

/// Parse a user-supplied blocklist entry. Accepts IPs (v4/v6, with or
/// without trailing `:port` / brackets) and returns the canonical IP string.
pub fn parse_block_target(raw: &str) -> Option<String> {
    let host = strip_port(raw);
    host.parse::<IpAddr>().ok().map(|ip| ip.to_string())
}

/// True when `addr` is a globally routable (external) address.
/// Handles zone IDs (`fe80::1%eth0`), ports, and brackets.
pub fn is_external_addr(addr: &str) -> bool {
    let clean = strip_port(addr);
    let host = clean.split('%').next().unwrap_or(clean);
    match host.parse::<IpAddr>() {
        Ok(IpAddr::V4(v)) => {
            !(v.is_loopback()
                || v.is_private()
                || v.is_link_local()
                || v.is_unspecified()
                || v.is_multicast()
                || v.is_broadcast())
        }
        Ok(IpAddr::V6(v)) => {
            if v.is_loopback() || v.is_unspecified() || v.is_multicast() {
                return false;
            }
            let o = v.octets();
            let ula = o[0] & 0xfe == 0xfc; // fc00::/7
            let link_local = o[0] == 0xfe && o[1] & 0xc0 == 0x80; // fe80::/10
            !(ula || link_local)
        }
        // Unparseable hostnames resolve via DNS to unknown addresses;
        // treat as non-external so heuristics stay quiet.
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_bare_ipv6_intact() {
        assert_eq!(strip_port("2001:db8::1"), "2001:db8::1");
        assert_eq!(strip_port("::1"), "::1");
        assert_eq!(strip_port("fe80::1"), "fe80::1");
    }

    #[test]
    fn strips_ports() {
        assert_eq!(strip_port("8.8.8.8:443"), "8.8.8.8");
        assert_eq!(strip_port("[2001:db8::1]:443"), "2001:db8::1");
        assert_eq!(strip_port("example.com:8080"), "example.com");
    }

    #[test]
    fn parses_block_targets() {
        assert_eq!(parse_block_target(" 8.8.8.8 ").as_deref(), Some("8.8.8.8"));
        assert_eq!(
            parse_block_target("2001:db8::1").as_deref(),
            Some("2001:db8::1")
        );
        assert_eq!(parse_block_target("not an ip"), None);
        assert_eq!(parse_block_target(""), None);
    }

    #[test]
    fn classifies_external() {
        assert!(!is_external_addr("127.0.0.1"));
        assert!(!is_external_addr("192.168.0.2"));
        assert!(!is_external_addr("10.0.0.1"));
        assert!(!is_external_addr("172.16.5.4"));
        assert!(!is_external_addr("::1"));
        assert!(!is_external_addr("fe80::1"));
        assert!(!is_external_addr("fe80::1%eth0"));
        assert!(!is_external_addr("fc00::1"));
        assert!(is_external_addr("2001:db8::1"));
        assert!(is_external_addr("[2001:db8::1]:443"));
        assert!(is_external_addr("8.8.8.8"));
        assert!(is_external_addr("8.8.8.8:443"));
    }
}

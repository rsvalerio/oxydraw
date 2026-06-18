//! Shared `Host`-header parsing for the crate's host trust checks.

/// Extract the host name from a `Host` header value: unwraps a bracketed IPv6
/// (`[::1]:3002` → `::1`), else strips a trailing `:port`. Both trust policies built on
/// top of this ([`crate::ext_routes`]'s loopback check for OAuth redirect_uris and
/// [`crate::frontend`]'s LAN check for asset rewrites) must parse identically — a
/// divergence would let an edge case pass one security gate but not the other.
pub(crate) fn host_name(host: &str) -> &str {
    match host.strip_prefix('[').and_then(|rest| rest.split_once(']')) {
        Some((v6, _)) => v6,
        None => host.rsplit_once(':').map_or(host, |(name, _)| name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_ports_and_brackets() {
        for (host, expected) in [
            ("localhost", "localhost"),
            ("localhost:3002", "localhost"),
            ("LOCALHOST:3002", "LOCALHOST"), // casing preserved; callers decide sensitivity
            ("127.0.0.1:3002", "127.0.0.1"),
            ("draw.example.com:443", "draw.example.com"),
            ("[::1]", "::1"),
            ("[::1]:3002", "::1"),
            ("[2001:DB8::1]:443", "2001:DB8::1"),
            ("", ""),
        ] {
            assert_eq!(host_name(host), expected, "host_name({host:?})");
        }
    }
}

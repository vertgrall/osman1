//! nettop / lsof socket parsing for structured connection identity.

use std::hash::{Hash, Hasher};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ConnectionId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SocketRole {
    Listener,
    Established,
    UdpFlow,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataSource {
    Nettop,
    Lsof,
    Merged,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Inbound,
    Outbound,
    Local,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddrFamily {
    V4,
    V6,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostPort {
    pub host: String,
    pub port: Option<u16>,
}

impl HostPort {
    pub fn display(&self) -> String {
        match self.port {
            Some(p) if is_wildcard_host(&self.host) => format!("*:{}", p),
            Some(p) => format!("{}:{}", self.host, p),
            None => self.host.clone(),
        }
    }

    pub fn is_any(&self) -> bool {
        is_wildcard_host(&self.host) && self.port.is_none()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedNettopSocket {
    pub transport: String,
    pub family: AddrFamily,
    pub local: HostPort,
    pub remote: HostPort,
    pub role: SocketRole,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedLsofSocket {
    pub transport: String,
    pub local: HostPort,
    pub remote: HostPort,
    pub role: SocketRole,
    pub state: String,
}

pub fn connection_id(
    pid: u32,
    transport: &str,
    local: &HostPort,
    remote: &HostPort,
) -> ConnectionId {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    pid.hash(&mut hasher);
    transport.hash(&mut hasher);
    local.host.hash(&mut hasher);
    local.port.hash(&mut hasher);
    remote.host.hash(&mut hasher);
    remote.port.hash(&mut hasher);
    ConnectionId(hasher.finish())
}

pub fn parse_nettop_key(key: &str) -> Option<ParsedNettopSocket> {
    let (proto, endpoints) = key.split_once(' ')?;
    let transport = normalize_transport(proto);
    let family = if proto.contains('6') {
        AddrFamily::V6
    } else if proto.contains('4') {
        AddrFamily::V4
    } else {
        AddrFamily::Unknown
    };

    let (local_raw, remote_raw) = if let Some((l, r)) = endpoints.split_once("<->") {
        (l.trim(), r.trim())
    } else {
        (endpoints.trim(), "*:*")
    };

    let local = parse_host_port(local_raw, family)?;
    let remote = parse_host_port(remote_raw, family)?;
    let role = nettop_role(&local, &remote, proto);

    Some(ParsedNettopSocket {
        transport,
        family,
        local,
        remote,
        role,
    })
}

pub fn parse_lsof_name(name: &str) -> Option<ParsedLsofSocket> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }

    let (node, state) = if let Some((left, state)) = trimmed.rsplit_once(" (") {
        let state = state.strip_suffix(')')?.to_string();
        (left.trim(), state)
    } else {
        (trimmed, "Open".to_string())
    };

    let (local_raw, remote_raw) = if let Some((l, r)) = node.split_once("->") {
        (l.trim(), Some(r.trim()))
    } else {
        (node, None)
    };

    let local = parse_host_port(local_raw, AddrFamily::Unknown)?;
    let remote = match remote_raw {
        Some(r) => parse_host_port(r, AddrFamily::Unknown)?,
        None => HostPort {
            host: "*".into(),
            port: None,
        },
    };

    let role = lsof_role(&state, &local, &remote);
    let transport = if trimmed.to_ascii_uppercase().contains("UDP") {
        "UDP".into()
    } else {
        "TCP".into()
    };

    Some(ParsedLsofSocket {
        transport,
        local,
        remote,
        role,
        state,
    })
}

pub fn is_process_key(key: &str) -> bool {
    key.contains('.')
        && !key.starts_with("tcp")
        && !key.starts_with("udp")
        && key
            .rsplit('.')
            .next()
            .and_then(|s| s.parse::<u32>().ok())
            .is_some()
}

pub fn parse_process_key(key: &str) -> Option<(String, u32)> {
    let (name, pid_str) = key.rsplit_once('.')?;
    let pid = pid_str.parse().ok()?;
    Some((name.to_string(), pid))
}

pub fn direction_for(role: SocketRole, remote: &HostPort) -> Direction {
    match role {
        SocketRole::Listener => Direction::Local,
        SocketRole::UdpFlow => Direction::Unknown,
        SocketRole::Established if is_wildcard_host(&remote.host) => Direction::Local,
        SocketRole::Established if is_loopback_host(&remote.host) || is_private_host(&remote.host) => {
            Direction::Local
        }
        SocketRole::Established => Direction::Outbound,
        SocketRole::Unknown => Direction::Unknown,
    }
}

pub fn is_private_host(host: &str) -> bool {
    if is_wildcard_host(host) || host == "0.0.0.0" {
        return true;
    }
    if is_loopback_host(host) {
        return true;
    }
    if host.starts_with("10.") {
        return true;
    }
    if host.starts_with("192.168.") {
        return true;
    }
    if let Some(rest) = host.strip_prefix("172.") {
        if let Some(octet) = rest.split('.').next() {
            if let Ok(n) = octet.parse::<u8>() {
                return (16..=31).contains(&n);
            }
        }
    }
    if host.starts_with("fe80:") || host.starts_with("fc") || host.starts_with("fd") {
        return true;
    }
    false
}

pub fn is_loopback_host(host: &str) -> bool {
    host == "localhost" || host == "127.0.0.1" || host == "::1" || host == "*"
}

pub fn is_wildcard_host(host: &str) -> bool {
    host == "*" || host == "*.*"
}

pub fn remote_display(remote: &HostPort, role: SocketRole) -> String {
    if role == SocketRole::Listener {
        return match remote.port {
            Some(p) => format!("listen :{}", p),
            None => "listen".into(),
        };
    }
    if remote.is_any() {
        return "—".into();
    }
    remote.display()
}

pub fn local_display(local: &HostPort) -> String {
    if local.is_any() {
        return "—".into();
    }
    local.display()
}

pub fn role_label(role: SocketRole) -> &'static str {
    match role {
        SocketRole::Listener => "Listen",
        SocketRole::Established => "Est",
        SocketRole::UdpFlow => "UDP",
        SocketRole::Unknown => "—",
    }
}

fn normalize_transport(proto: &str) -> String {
    if proto.starts_with("udp") {
        "UDP".into()
    } else {
        "TCP".into()
    }
}

fn nettop_role(local: &HostPort, remote: &HostPort, proto: &str) -> SocketRole {
    if proto.starts_with("udp") {
        return SocketRole::UdpFlow;
    }
    if is_wildcard_host(&remote.host) || remote.host == "*.*" {
        return SocketRole::Listener;
    }
    SocketRole::Established
}

fn lsof_role(state: &str, local: &HostPort, remote: &HostPort) -> SocketRole {
    let upper = state.to_ascii_uppercase();
    if upper.contains("LISTEN") {
        return SocketRole::Listener;
    }
    if upper.contains("ESTABLISHED") {
        return SocketRole::Established;
    }
    if is_wildcard_host(&remote.host) {
        SocketRole::Listener
    } else {
        SocketRole::Unknown
    }
}

fn parse_host_port(raw: &str, family: AddrFamily) -> Option<HostPort> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    if raw == "*" || raw == "*.*" || raw == "*:*" {
        return Some(HostPort {
            host: "*".into(),
            port: None,
        });
    }

    if let Some(stripped) = raw.strip_prefix('[') {
        if let Some((host, port_str)) = stripped.split_once("]:") {
            let port = port_str.parse().ok();
            return Some(HostPort {
                host: host.to_string(),
                port,
            });
        }
    }

    if raw.contains('.') && raw.contains(':') && !raw.contains("->") {
        // IPv6 nettop shorthand: ::1.631
        if raw.starts_with(':') {
            if let Some((host, port_str)) = raw.rsplit_once('.') {
                if port_str.chars().all(|c| c.is_ascii_digit()) {
                    return Some(HostPort {
                        host: host.to_string(),
                        port: port_str.parse().ok(),
                    });
                }
            }
        }
    }

    if let Some((host, port_str)) = raw.rsplit_once(':') {
        if !host.is_empty() && port_str.chars().all(|c| c.is_ascii_digit()) {
            return Some(HostPort {
                host: host.to_string(),
                port: port_str.parse().ok(),
            });
        }
    }

    if raw.starts_with('*') {
        if let Some(port_str) = raw.strip_prefix("*:") {
            return Some(HostPort {
                host: "*".into(),
                port: port_str.parse().ok(),
            });
        }
    }

    if family == AddrFamily::V6 && raw.contains(':') {
        return Some(HostPort {
            host: raw.to_string(),
            port: None,
        });
    }

    Some(HostPort {
        host: raw.to_string(),
        port: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tcp_established_v4() {
        let parsed = parse_nettop_key("tcp4 192.168.1.86:62712<->104.18.18.125:443").unwrap();
        assert_eq!(parsed.local.host, "192.168.1.86");
        assert_eq!(parsed.local.port, Some(62712));
        assert_eq!(parsed.remote.host, "104.18.18.125");
        assert_eq!(parsed.remote.port, Some(443));
        assert_eq!(parsed.role, SocketRole::Established);
        assert!(!is_private_host(&parsed.remote.host));
    }

    #[test]
    fn parse_tcp_listener_wildcard() {
        let parsed = parse_nettop_key("tcp4 *:7000<->*:*").unwrap();
        assert_eq!(parsed.local.port, Some(7000));
        assert_eq!(parsed.role, SocketRole::Listener);
        assert_eq!(remote_display(&parsed.remote, parsed.role), "listen");
    }

    #[test]
    fn parse_tcp6_loopback_listener() {
        let parsed = parse_nettop_key("tcp6 ::1.631<->*.*").unwrap();
        assert_eq!(parsed.local.host, "::1");
        assert_eq!(parsed.local.port, Some(631));
        assert_eq!(parsed.family, AddrFamily::V6);
        assert_eq!(parsed.role, SocketRole::Listener);
    }

    #[test]
    fn parse_udp_flow() {
        let parsed = parse_nettop_key("udp4 *:57427<->*:*").unwrap();
        assert_eq!(parsed.transport, "UDP");
        assert_eq!(parsed.role, SocketRole::UdpFlow);
    }

    #[test]
    fn parse_lsof_established() {
        let parsed =
            parse_lsof_name("192.168.1.86:62712->104.18.18.125:443 (ESTABLISHED)").unwrap();
        assert_eq!(parsed.remote.host, "104.18.18.125");
        assert_eq!(parsed.remote.port, Some(443));
        assert_eq!(parsed.role, SocketRole::Established);
    }

    #[test]
    fn parse_lsof_listen() {
        let parsed = parse_lsof_name("*:7000 (LISTEN)").unwrap();
        assert_eq!(parsed.local.port, Some(7000));
        assert_eq!(parsed.role, SocketRole::Listener);
    }

    #[test]
    fn connection_id_is_stable() {
        let local = HostPort {
            host: "10.0.0.2".into(),
            port: Some(443),
        };
        let remote = HostPort {
            host: "1.1.1.1".into(),
            port: Some(443),
        };
        let a = connection_id(42, "TCP", &local, &remote);
        let b = connection_id(42, "TCP", &local, &remote);
        assert_eq!(a, b);
    }
}

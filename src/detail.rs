use std::collections::{HashMap, HashSet};
use std::process::Command;

use sysinfo::Networks;

use crate::network::InterfaceStats;
use crate::parse::{
    connection_id, direction_for, is_loopback_host, is_private_host, is_process_key,
    local_display, parse_lsof_name, parse_nettop_key, parse_process_key, remote_display,
    role_label, ConnectionId, DataSource, Direction, HostPort, SocketRole,
};
use crate::theme::{format_rate, format_total};

#[derive(Clone, Debug, Default)]
pub struct TrafficSnapshot {
    pub connections: Vec<ConnectionDetail>,
    pub processes: Vec<ProcessTraffic>,
}

impl TrafficSnapshot {
    pub fn collect() -> Self {
        let mut snapshot = Self::default();
        collect_nettop(None, &mut snapshot);
        append_lsof_listeners(&mut snapshot);
        snapshot.finalize();
        snapshot
    }

    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    pub fn for_interface(&self, interface: &str) -> (Vec<ProcessTraffic>, Vec<ConnectionDetail>) {
        let connections: Vec<_> = self
            .connections
            .iter()
            .filter(|c| c.interface == interface)
            .cloned()
            .collect();

        let mut per_pid: HashMap<u32, ProcessTraffic> = HashMap::new();
        for conn in &connections {
            let entry = per_pid.entry(conn.pid).or_insert_with(|| ProcessTraffic {
                name: conn.process_name.clone(),
                pid: conn.pid,
                rx_bytes: 0,
                tx_bytes: 0,
                connection_count: 0,
            });
            entry.rx_bytes = entry.rx_bytes.saturating_add(conn.rx_bytes);
            entry.tx_bytes = entry.tx_bytes.saturating_add(conn.tx_bytes);
            entry.connection_count += 1;
        }

        let mut processes: Vec<_> = per_pid.into_values().collect();
        processes.sort_by(|a, b| b.combined_bytes().cmp(&a.combined_bytes()));
        processes.truncate(24);

        let mut ranked = connections;
        ranked.sort_by(|a, b| b.combined_bytes().cmp(&a.combined_bytes()));
        ranked.truncate(40);

        (processes, ranked)
    }

    fn finalize(&mut self) {
        self.processes.sort_by(|a, b| b.combined_bytes().cmp(&a.combined_bytes()));
        self.processes.truncate(80);

        self.connections
            .sort_by(|a, b| b.combined_bytes().cmp(&a.combined_bytes()));
        self.connections.truncate(120);
    }
}

#[derive(Clone, Debug, Default)]
pub struct InterfaceDetail {
    pub interface: String,
    pub stats: Option<InterfaceStats>,
    pub mac: String,
    pub addresses: Vec<String>,
    pub mtu: u64,
    pub packets_in: u64,
    pub packets_out: u64,
    pub errors: u64,
    pub drops: u64,
    pub processes: Vec<ProcessTraffic>,
    pub connections: Vec<ConnectionDetail>,
    pub note: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ProcessTraffic {
    pub name: String,
    pub pid: u32,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub connection_count: usize,
}

impl ProcessTraffic {
    pub fn combined_bytes(&self) -> u64 {
        self.rx_bytes.saturating_add(self.tx_bytes)
    }
}

#[derive(Clone, Debug)]
pub struct ConnectionDetail {
    pub id: ConnectionId,
    pub process_name: String,
    pub pid: u32,
    pub interface: String,
    pub protocol: String,
    pub transport: String,
    pub endpoint: String,
    pub state: String,
    pub local_host: String,
    pub local_port: Option<u16>,
    pub remote_host: String,
    pub remote_port: Option<u16>,
    pub role: SocketRole,
    pub direction: Direction,
    pub remote_is_private: bool,
    pub remote_is_loopback: bool,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub source: DataSource,
}

impl ConnectionDetail {
    pub fn combined_bytes(&self) -> u64 {
        self.rx_bytes.saturating_add(self.tx_bytes)
    }

    pub fn remote_label(&self) -> String {
        remote_display(
            &HostPort {
                host: self.remote_host.clone(),
                port: self.remote_port,
            },
            self.role,
        )
    }

    pub fn local_label(&self) -> String {
        local_display(&HostPort {
            host: self.local_host.clone(),
            port: self.local_port,
        })
    }

    pub fn role_label(&self) -> &'static str {
        role_label(self.role)
    }

    pub fn direction_label(&self) -> &'static str {
        match self.direction {
            Direction::Inbound => "Inbound",
            Direction::Outbound => "Outbound",
            Direction::Local => "Local",
            Direction::Unknown => "—",
        }
    }

    pub fn matches_filter(&self, needle: &str) -> bool {
        if needle.is_empty() {
            return true;
        }
        let hay = format!(
            "{} {} {} {} {} {} {}",
            self.process_name,
            self.remote_label(),
            self.local_label(),
            self.transport,
            self.role_label(),
            self.state,
            self.interface
        )
        .to_ascii_lowercase();
        hay.contains(needle)
    }
}

pub fn interface_detail_from_traffic(
    interface: &str,
    snapshot: &crate::network::NetworkSnapshot,
    traffic: &TrafficSnapshot,
) -> InterfaceDetail {
    let stats = snapshot
        .interfaces
        .iter()
        .find(|i| i.name == interface)
        .cloned();

    let networks = Networks::new_with_refreshed_list();
    let (mac, addresses, mtu, packets_in, packets_out, errors, drops) =
        interface_hardware(&networks, interface);

    let (processes, connections) = traffic.for_interface(interface);

    let note = if processes.is_empty() && connections.is_empty() {
        Some("No active sockets matched this interface in the latest sample.".into())
    } else {
        None
    };

    InterfaceDetail {
        interface: interface.to_string(),
        stats,
        mac,
        addresses,
        mtu,
        packets_in,
        packets_out,
        errors,
        drops,
        processes,
        connections,
        note,
    }
}

pub fn load_system_traffic() -> (Vec<ProcessTraffic>, Vec<ConnectionDetail>) {
    let snapshot = TrafficSnapshot::collect();
    (snapshot.processes, snapshot.connections)
}

fn interface_hardware(
    networks: &Networks,
    interface: &str,
) -> (String, Vec<String>, u64, u64, u64, u64, u64) {
    let Some(data) = networks.list().get(interface) else {
        return ("—".into(), Vec::new(), 0, 0, 0, 0, 0);
    };

    let mac = data.mac_address().to_string();
    let addresses = data
        .ip_networks()
        .iter()
        .map(|net| net.addr.to_string())
        .collect();

    let errors = data
        .total_errors_on_received()
        .saturating_add(data.total_errors_on_transmitted());

    (
        mac,
        addresses,
        data.mtu(),
        data.packets_received(),
        data.packets_transmitted(),
        errors,
        0,
    )
}

fn collect_nettop(interface: Option<&str>, snapshot: &mut TrafficSnapshot) {
    let mut per_process: HashMap<u32, ProcessTraffic> = HashMap::new();
    let mut current: Option<(String, u32)> = None;

    for mode in ["tcp", "udp"] {
        let output = Command::new("nettop")
            .args(["-m", mode, "-L", "1", "-n", "-x"])
            .output();

        let Ok(output) = output else { continue };
        let text = String::from_utf8_lossy(&output.stdout);

        for line in text.lines().skip(1) {
            let cols: Vec<&str> = line.split(',').collect();
            if cols.len() < 6 {
                continue;
            }

            let key = cols[1].trim();
            if is_process_key(key) {
                current = parse_process_key(key);
                continue;
            }

            if !(key.starts_with("tcp") || key.starts_with("udp")) {
                continue;
            }

            let iface = cols.get(2).map(|s| s.trim()).unwrap_or("");
            if interface.is_some_and(|wanted| iface != wanted) {
                continue;
            }

            let parsed = parse_nettop_key(key);
            let state = cols.get(3).unwrap_or(&"").trim().to_string();
            let rx = cols.get(4).copied().and_then(parse_u64).unwrap_or(0);
            let tx = cols.get(5).copied().and_then(parse_u64).unwrap_or(0);
            let (process_name, pid) = current.clone().unwrap_or(("unknown".into(), 0));

            let conn = build_connection_from_nettop(
                key,
                &parsed,
                process_name.clone(),
                pid,
                iface,
                state,
                rx,
                tx,
            );

            snapshot.connections.push(conn);

            let entry = per_process.entry(pid).or_insert_with(|| ProcessTraffic {
                name: process_name,
                pid,
                rx_bytes: 0,
                tx_bytes: 0,
                connection_count: 0,
            });
            entry.rx_bytes = entry.rx_bytes.saturating_add(rx);
            entry.tx_bytes = entry.tx_bytes.saturating_add(tx);
            entry.connection_count += 1;
        }
    }

    snapshot.processes = per_process.into_values().collect();
}

fn build_connection_from_nettop(
    key: &str,
    parsed: &Option<crate::parse::ParsedNettopSocket>,
    process_name: String,
    pid: u32,
    iface: &str,
    state: String,
    rx: u64,
    tx: u64,
) -> ConnectionDetail {
    let (transport, protocol, local, remote, role) = if let Some(parsed) = parsed {
        (
            parsed.transport.clone(),
            key.split_whitespace().next().unwrap_or("tcp").to_string(),
            parsed.local.clone(),
            parsed.remote.clone(),
            parsed.role,
        )
    } else {
        (
            "TCP".into(),
            key.split_whitespace().next().unwrap_or("tcp").to_string(),
            HostPort {
                host: "—".into(),
                port: None,
            },
            HostPort {
                host: "—".into(),
                port: None,
            },
            SocketRole::Unknown,
        )
    };

    let direction = direction_for(role, &remote);
    let remote_is_private = is_private_host(&remote.host);
    let remote_is_loopback = is_loopback_host(&remote.host);
    let id = connection_id(pid, &transport, &local, &remote);

    ConnectionDetail {
        id,
        process_name,
        pid,
        interface: iface.to_string(),
        protocol,
        transport,
        endpoint: key.to_string(),
        state,
        local_host: local.host,
        local_port: local.port,
        remote_host: remote.host,
        remote_port: remote.port,
        role,
        direction,
        remote_is_private,
        remote_is_loopback,
        rx_bytes: rx,
        tx_bytes: tx,
        source: DataSource::Nettop,
    }
}

fn append_lsof_listeners(snapshot: &mut TrafficSnapshot) {
    let output = Command::new("lsof").args(["-n", "-P", "-i"]).output();
    let Ok(output) = output else { return };
    let text = String::from_utf8_lossy(&output.stdout);
    let networks = Networks::new_with_refreshed_list();

    let mut seen: HashSet<(u32, Option<u16>, String)> = snapshot
        .connections
        .iter()
        .map(|c| (c.pid, c.local_port, c.transport.clone()))
        .collect();

    for line in text.lines().skip(1) {
        let Some((process_name, pid, node)) = parse_lsof_line(line) else {
            continue;
        };

        let Some(parsed) = parse_lsof_name(&node) else {
            continue;
        };

        if parsed.role != SocketRole::Listener {
            continue;
        }

        let dedupe_key = (pid, parsed.local.port, parsed.transport.clone());
        if seen.contains(&dedupe_key) {
            continue;
        }
        seen.insert(dedupe_key);

        let local = parsed.local.clone();
        let remote = parsed.remote.clone();
        let role = parsed.role;
        let direction = direction_for(role, &remote);
        let id = connection_id(pid, &parsed.transport, &local, &remote);
        let interface = resolve_interface(&networks, &local);

        let remote_is_private = is_private_host(&remote.host);
        let remote_is_loopback = is_loopback_host(&remote.host);

        snapshot.connections.push(ConnectionDetail {
            id,
            process_name: process_name.clone(),
            pid,
            interface,
            protocol: parsed.transport.to_ascii_lowercase(),
            transport: parsed.transport.clone(),
            endpoint: node,
            state: parsed.state,
            local_host: local.host,
            local_port: local.port,
            remote_host: remote.host,
            remote_port: remote.port,
            role,
            direction,
            remote_is_private,
            remote_is_loopback,
            rx_bytes: 0,
            tx_bytes: 0,
            source: DataSource::Lsof,
        });

        if let Some(entry) = snapshot.processes.iter_mut().find(|p| p.pid == pid) {
            entry.connection_count += 1;
        } else {
            snapshot.processes.push(ProcessTraffic {
                name: process_name,
                pid,
                rx_bytes: 0,
                tx_bytes: 0,
                connection_count: 1,
            });
        }
    }
}

fn resolve_interface(networks: &Networks, local: &crate::parse::HostPort) -> String {
    let host = local.host.as_str();
    if host == "—" {
        return "—".into();
    }
    if host == "*" || host == "0.0.0.0" {
        return default_interface(networks);
    }

    for (name, data) in networks.list() {
        if name.starts_with("lo") {
            continue;
        }
        for net in data.ip_networks() {
            if net.addr.to_string() == host {
                return name.clone();
            }
        }
    }

    if host == "127.0.0.1" || host == "::1" || host == "localhost" {
        return networks
            .list()
            .keys()
            .find(|name| name.starts_with("lo"))
            .cloned()
            .unwrap_or_else(|| "—".into());
    }

    "—".into()
}

fn default_interface(networks: &Networks) -> String {
    if networks.list().contains_key("en0") {
        return "en0".into();
    }
    networks
        .list()
        .keys()
        .find(|name| !name.starts_with("lo") && !name.starts_with("awdl"))
        .cloned()
        .unwrap_or_else(|| "—".into())
}

fn parse_lsof_line(line: &str) -> Option<(String, u32, String)> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 9 {
        return None;
    }
    let name = parts[0].to_string();
    let pid = parts[1].parse().ok()?;
    let node = parts[8..].join(" ");
    Some((name, pid, node))
}

fn parse_u64(value: &str) -> Option<u64> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    trimmed.parse().ok()
}

impl InterfaceDetail {
    pub fn ipv4(&self) -> String {
        self.addresses
            .iter()
            .find(|a| a.contains('.'))
            .cloned()
            .unwrap_or_else(|| "—".into())
    }

    pub fn ipv6(&self) -> String {
        self.addresses
            .iter()
            .find(|a| a.contains(':'))
            .cloned()
            .unwrap_or_else(|| "—".into())
    }

    pub fn status_label(&self) -> &'static str {
        if self.stats.as_ref().is_some_and(|s| s.is_active()) {
            "Connected"
        } else {
            "Inactive"
        }
    }

    pub fn consistency_label(&self) -> String {
        self.stats
            .as_ref()
            .map(|s| format!("{:.0}% steady", s.consistency * 100.0))
            .unwrap_or_else(|| "—".into())
    }

    pub fn live_rates(&self) -> String {
        let Some(stats) = &self.stats else {
            return "—".into();
        };
        format!(
            "↓ {}  ↑ {}  ∑ {}",
            format_rate(stats.rx_bps),
            format_rate(stats.tx_bps),
            format_rate(stats.combined_bps)
        )
    }

    pub fn session_totals(&self) -> String {
        let Some(stats) = &self.stats else {
            return "—".into();
        };
        format!(
            "↓ {}  ↑ {}",
            format_total(stats.total_rx),
            format_total(stats.total_tx)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::HostPort;
    use sysinfo::Networks;

    #[test]
    fn default_interface_prefers_en0() {
        let networks = Networks::new_with_refreshed_list();
        let iface = default_interface(&networks);
        assert!(!iface.is_empty());
    }

    #[test]
    fn resolve_interface_matches_local_ip() {
        let networks = Networks::new_with_refreshed_list();
        let local = HostPort {
            host: "127.0.0.1".into(),
            port: Some(8080),
        };
        let iface = resolve_interface(&networks, &local);
        assert!(iface.starts_with("lo") || iface == "—");
    }

    #[test]
    fn resolve_wildcard_uses_default_interface() {
        let networks = Networks::new_with_refreshed_list();
        let local = HostPort {
            host: "*".into(),
            port: Some(7000),
        };
        let iface = resolve_interface(&networks, &local);
        assert_ne!(iface, "—");
    }
}

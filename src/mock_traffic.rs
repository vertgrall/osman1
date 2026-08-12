//! Realistic demo traffic for README screenshots and UI previews.

use std::time::{Duration, Instant};

use crate::detail::{ConnectionDetail, ProcessTraffic, TrafficSnapshot};
use crate::network::{InterfaceStats, NetworkSnapshot};
use crate::parse::{ConnectionId, DataSource, Direction, SocketRole};
use crate::rate_tracker::LiveConnectionRate;

const HISTORY_LEN: usize = 90;

/// Wave-shaped bps series — reads like live adapter traffic on the hero chart.
fn wave_series(len: usize, base: f64, amplitude: f64, phase: f64) -> Vec<f64> {
    (0..len)
        .map(|i| {
            let t = i as f64 / len as f64 * std::f64::consts::TAU * 2.8 + phase;
            let wobble = t.sin() * 0.5 + (t * 1.9).sin() * 0.32 + (t * 0.55).cos() * 0.18;
            (base + amplitude * wobble).max(base * 0.22)
        })
        .collect()
}

fn iface(
    name: &str,
    rx_base: f64,
    tx_base: f64,
    phase: f64,
    total_rx: u64,
    total_tx: u64,
) -> InterfaceStats {
    let rx_history = wave_series(HISTORY_LEN, rx_base, rx_base * 0.38, phase);
    let tx_history = wave_series(HISTORY_LEN, tx_base, tx_base * 0.42, phase + 1.1);
    let combined_history: Vec<f64> = rx_history
        .iter()
        .zip(&tx_history)
        .map(|(r, t)| r + t)
        .collect();
    let rx_bps = *rx_history.last().unwrap();
    let tx_bps = *tx_history.last().unwrap();
    let combined_bps = rx_bps + tx_bps;
    let avg = combined_history.iter().sum::<f64>() / combined_history.len() as f64;
    let variance = combined_history
        .iter()
        .map(|v| (v - avg).powi(2))
        .sum::<f64>()
        / combined_history.len() as f64;
    let consistency = if avg > 0.0 {
        (1.0 - (variance.sqrt() / avg).min(1.0)).max(0.0)
    } else {
        0.0
    };

    InterfaceStats {
        name: name.into(),
        rx_bps,
        tx_bps,
        combined_bps,
        total_rx,
        total_tx,
        consistency,
        heavy_consistent: combined_bps >= 40_000.0 && consistency >= 0.52,
        rx_history,
        tx_history,
        combined_history,
    }
}

/// Snapshot with hero chart + adapter sparklines filled in.
pub fn network_snapshot() -> NetworkSnapshot {
    let interfaces = vec![
        iface("en0", 5_400_000.0, 1_650_000.0, 0.0, 2_840_000_000, 680_000_000),
        iface("en5", 920_000.0, 410_000.0, 2.4, 128_000_000, 44_000_000),
        iface("utun3", 380_000.0, 360_000.0, 4.8, 52_000_000, 48_000_000),
        iface("bridge0", 48_000.0, 12_000.0, 6.2, 8_400_000, 1_200_000),
    ];

    let rx_history = wave_series(HISTORY_LEN, 6_748_000.0, 2_100_000.0, 0.15);
    let tx_history = wave_series(HISTORY_LEN, 2_432_000.0, 780_000.0, 1.05);
    let combined_history: Vec<f64> = rx_history
        .iter()
        .zip(&tx_history)
        .map(|(r, t)| r + t)
        .collect();

    NetworkSnapshot {
        total_rx_bps: *rx_history.last().unwrap(),
        total_tx_bps: *tx_history.last().unwrap(),
        interfaces,
        rx_history,
        tx_history,
        combined_history,
        process_count: 186,
        connection_count: 42,
        sample_tick: 1_284,
    }
}

fn conn(
    id: u64,
    process: &str,
    pid: u32,
    iface: &str,
    remote: &str,
    port: u16,
    rx: u64,
    tx: u64,
) -> ConnectionDetail {
    ConnectionDetail {
        id: ConnectionId(id),
        process_name: process.into(),
        pid,
        interface: iface.into(),
        protocol: "tcp".into(),
        transport: "TCP".into(),
        endpoint: format!("{remote}:{port}"),
        state: "ESTABLISHED".into(),
        local_host: "192.168.1.42".into(),
        local_port: Some(49_000 + (id as u16 % 800)),
        remote_host: remote.into(),
        remote_port: Some(port),
        role: SocketRole::Established,
        direction: Direction::Outbound,
        remote_is_private: false,
        remote_is_loopback: false,
        rx_bytes: rx,
        tx_bytes: tx,
        source: DataSource::Nettop,
    }
}

pub fn traffic_snapshot() -> TrafficSnapshot {
    let connections = vec![
        conn(1, "Google Chrome", 8842, "en0", "142.250.80.78", 443, 842_000_000, 96_000_000),
        conn(2, "Cursor", 7721, "en0", "api2.cursor.sh", 443, 128_000_000, 42_000_000),
        conn(3, "Slack", 6610, "en0", "wss-primary.slack.com", 443, 64_000_000, 18_000_000),
        conn(4, "Spotify", 5599, "en0", "audio-sp-bos5.spotify.com", 443, 220_000_000, 8_400_000),
        conn(5, "cloudflared", 4401, "utun3", "162.159.134.234", 7844, 18_000_000, 16_000_000),
        conn(6, "Docker Desktop", 3310, "en0", "registry-1.docker.io", 443, 52_000_000, 12_000_000),
        conn(7, "Messages", 2298, "en0", "17.188.194.47", 5223, 9_800_000, 4_200_000),
        conn(8, "Dropbox", 1188, "en0", "104.244.42.1", 443, 31_000_000, 6_800_000),
        conn(9, "Homebrew", 991, "en0", "ghcr.io", 443, 4_200_000, 980_000),
        conn(10, "osman1", 20839, "en0", "127.0.0.1", 8080, 120_000, 88_000),
    ];

    let mut per_pid: std::collections::HashMap<u32, ProcessTraffic> =
        std::collections::HashMap::new();
    for c in &connections {
        let entry = per_pid.entry(c.pid).or_insert_with(|| ProcessTraffic {
            name: c.process_name.clone(),
            pid: c.pid,
            rx_bytes: 0,
            tx_bytes: 0,
            connection_count: 0,
        });
        entry.rx_bytes = entry.rx_bytes.saturating_add(c.rx_bytes);
        entry.tx_bytes = entry.tx_bytes.saturating_add(c.tx_bytes);
        entry.connection_count += 1;
    }
    let mut processes: Vec<_> = per_pid.into_values().collect();
    processes.sort_by(|a, b| b.combined_bytes().cmp(&a.combined_bytes()));

    TrafficSnapshot {
        connections,
        processes,
    }
}

pub fn live_rates(connections: &[ConnectionDetail]) -> Vec<LiveConnectionRate> {
    let presets: &[(f64, f64)] = &[
        (3_200_000.0, 420_000.0),
        (840_000.0, 310_000.0),
        (520_000.0, 180_000.0),
        (680_000.0, 42_000.0),
        (290_000.0, 260_000.0),
        (410_000.0, 95_000.0),
        (88_000.0, 36_000.0),
        (210_000.0, 48_000.0),
        (64_000.0, 12_000.0),
        (8_400.0, 2_200.0),
    ];

    connections
        .iter()
        .enumerate()
        .map(|(i, conn)| {
            let (rx_bps, tx_bps) = presets.get(i).copied().unwrap_or((24_000.0, 8_000.0));
            LiveConnectionRate {
                connection_id: conn.id,
                process_name: conn.process_name.clone(),
                remote_label: conn.remote_label(),
                local_label: conn.local_label(),
                interface: conn.interface.clone(),
                rx_bps,
                tx_bps,
            }
        })
        .collect()
}

pub fn demo_started_at() -> Instant {
    Instant::now() - Duration::from_secs(2 * 3600 + 14 * 60 + 38)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_snapshot_has_chart_history() {
        let snap = network_snapshot();
        assert!(snap.rx_history.len() >= 60);
        assert!(snap.total_rx_bps > 1_000_000.0);
        assert!(snap.interfaces.iter().any(|i| i.combined_bps > 1_000_000.0));
    }

    #[test]
    fn demo_traffic_has_connections_and_rates() {
        let traffic = traffic_snapshot();
        assert!(traffic.connections.len() >= 8);
        let rates = live_rates(&traffic.connections);
        assert!(rates.iter().any(|r| r.combined_bps() > 500_000.0));
    }
}

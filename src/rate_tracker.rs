use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::detail::ConnectionDetail;
use crate::network::IFACE_HISTORY_LEN;
use crate::parse::ConnectionId;

#[derive(Clone, Debug, Default)]
struct ConnectionHistory {
    rx: Vec<f64>,
    tx: Vec<f64>,
    combined: Vec<f64>,
}

impl ConnectionHistory {
    fn push(&mut self, rx: f64, tx: f64) {
        self.rx.push(rx);
        self.tx.push(tx);
        self.combined.push(rx + tx);
        for series in [&mut self.rx, &mut self.tx, &mut self.combined] {
            if series.len() > IFACE_HISTORY_LEN {
                series.remove(0);
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct ConnectionTrafficHistory {
    pub rx: Vec<f64>,
    pub tx: Vec<f64>,
    pub combined: Vec<f64>,
}

#[derive(Clone, Debug)]
pub struct LiveConnectionRate {
    pub connection_id: ConnectionId,
    pub process_name: String,
    pub remote_label: String,
    pub local_label: String,
    pub interface: String,
    pub rx_bps: f64,
    pub tx_bps: f64,
}

impl LiveConnectionRate {
    pub fn combined_bps(&self) -> f64 {
        self.rx_bps + self.tx_bps
    }
}

#[derive(Clone, Default)]
pub struct RateTracker {
    prev: HashMap<u64, (u64, u64)>,
    first_seen: HashMap<u64, Instant>,
    histories: HashMap<u64, ConnectionHistory>,
}

impl RateTracker {
    pub fn update(
        &mut self,
        connections: &[ConnectionDetail],
        interval: Duration,
    ) -> Vec<LiveConnectionRate> {
        let secs = interval.as_secs_f64().max(0.001);
        let mut rates = Vec::with_capacity(connections.len());
        let now = Instant::now();

        for conn in connections {
            let key = conn.id.0;
            let total_rx = conn.rx_bytes;
            let total_tx = conn.tx_bytes;

            self.first_seen.entry(key).or_insert(now);

            let (rx_bps, tx_bps) = if let Some((prev_rx, prev_tx)) = self.prev.get(&key) {
                (
                    total_rx.saturating_sub(*prev_rx) as f64 / secs,
                    total_tx.saturating_sub(*prev_tx) as f64 / secs,
                )
            } else {
                (0.0, 0.0)
            };

            self.prev.insert(key, (total_rx, total_tx));
            self.histories
                .entry(key)
                .or_default()
                .push(rx_bps, tx_bps);

            rates.push(LiveConnectionRate {
                connection_id: conn.id,
                process_name: conn.process_name.clone(),
                remote_label: conn.remote_label(),
                local_label: conn.local_label(),
                interface: conn.interface.clone(),
                rx_bps,
                tx_bps,
            });
        }

        rates.sort_by(|a, b| {
            b.combined_bps()
                .partial_cmp(&a.combined_bps())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        rates
    }

    pub fn session_age(&self, id: ConnectionId) -> Option<Duration> {
        self.first_seen.get(&id.0).map(|start| start.elapsed())
    }

    pub fn connection_history(&self, id: ConnectionId) -> Option<ConnectionTrafficHistory> {
        self.histories.get(&id.0).map(|h| ConnectionTrafficHistory {
            rx: h.rx.clone(),
            tx: h.tx.clone(),
            combined: h.combined.clone(),
        })
    }
}

pub fn rates_for_process(rates: &[LiveConnectionRate], process: &str) -> (f64, f64) {
    let mut rx = 0.0;
    let mut tx = 0.0;
    for r in rates.iter().filter(|r| r.process_name == process) {
        rx += r.rx_bps;
        tx += r.tx_bps;
    }
    (rx, tx)
}

pub fn rates_for_interface(
    rates: &[LiveConnectionRate],
    interface: &str,
) -> Vec<LiveConnectionRate> {
    rates
        .iter()
        .filter(|r| r.interface == interface)
        .cloned()
        .collect()
}

pub fn rate_for_connection(
    rates: &[LiveConnectionRate],
    id: ConnectionId,
) -> Option<&LiveConnectionRate> {
    rates.iter().find(|r| r.connection_id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::{ConnectionId, DataSource, Direction, SocketRole};

    fn sample_conn(id: u64, rx: u64, tx: u64) -> ConnectionDetail {
        ConnectionDetail {
            id: ConnectionId(id),
            process_name: "curl".into(),
            pid: 42,
            interface: "en0".into(),
            protocol: "tcp".into(),
            transport: "tcp".into(),
            endpoint: "example.com:443".into(),
            state: "ESTABLISHED".into(),
            local_host: "10.0.0.2".into(),
            local_port: Some(50123),
            remote_host: "93.184.216.34".into(),
            remote_port: Some(443),
            role: SocketRole::Established,
            direction: Direction::Outbound,
            remote_is_private: false,
            remote_is_loopback: false,
            rx_bytes: rx,
            tx_bytes: tx,
            source: DataSource::Nettop,
        }
    }

    #[test]
    fn connection_history_accumulates_samples() {
        let mut tracker = RateTracker::default();
        let interval = Duration::from_secs(1);
        let id = ConnectionId(7);

        tracker.update(&[sample_conn(7, 1000, 500)], interval);
        tracker.update(&[sample_conn(7, 2000, 900)], interval);

        let history = tracker.connection_history(id).expect("history");
        assert_eq!(history.rx.len(), 2);
        assert_eq!(history.tx.len(), 2);
        assert!(history.combined[1] > history.combined[0]);
    }
}

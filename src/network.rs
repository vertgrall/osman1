use std::collections::HashMap;
use std::time::Duration;

use sysinfo::{Networks, System};

use crate::theme::format_total;

pub const HISTORY_LEN: usize = 900;
pub const IFACE_HISTORY_LEN: usize = 900;
const HEAVY_RATE_BPS: f64 = 40_000.0;
const CONSISTENCY_FLOOR: f64 = 0.52;

#[derive(Clone, Debug, Default)]
struct IfaceHistories {
    rx: Vec<f64>,
    tx: Vec<f64>,
    combined: Vec<f64>,
}

impl IfaceHistories {
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

#[derive(Clone, Debug, Default)]
pub struct NetworkSnapshot {
    pub interfaces: Vec<InterfaceStats>,
    pub rx_history: Vec<f64>,
    pub tx_history: Vec<f64>,
    pub combined_history: Vec<f64>,
    pub total_rx_bps: f64,
    pub total_tx_bps: f64,
    pub process_count: usize,
    pub connection_count: usize,
    /// Monotonic counter — bumps every poll so charts keep repainting.
    pub sample_tick: u64,
}

#[derive(Clone, Debug)]
pub struct InterfaceStats {
    pub name: String,
    pub rx_bps: f64,
    pub tx_bps: f64,
    pub combined_bps: f64,
    pub total_rx: u64,
    pub total_tx: u64,
    pub consistency: f64,
    pub heavy_consistent: bool,
    pub rx_history: Vec<f64>,
    pub tx_history: Vec<f64>,
    pub combined_history: Vec<f64>,
}

impl InterfaceStats {
    pub fn subtitle(&self) -> String {
        format!(
            "Since launch ↓ {}  ↑ {}",
            format_total(self.total_rx),
            format_total(self.total_tx)
        )
    }

    pub fn is_active(&self) -> bool {
        self.combined_bps > 1.0 || self.total_rx > 0 || self.total_tx > 0
    }

    pub fn status_label(&self) -> &'static str {
        if self.is_active() {
            "Connected"
        } else {
            "Inactive"
        }
    }
}

#[derive(Default)]
pub struct NetworkTracker {
    iface_histories: HashMap<String, IfaceHistories>,
}

impl NetworkTracker {
    pub fn sample(&mut self, networks: &mut Networks, connection_count: usize) -> NetworkSnapshot {
        networks.refresh(true);

        let mut interfaces = Vec::new();
        let mut total_rx_bps = 0.0;
        let mut total_tx_bps = 0.0;

        for (name, data) in networks.iter() {
            if name.starts_with("lo") {
                continue;
            }

            let rx_bps = data.received() as f64;
            let tx_bps = data.transmitted() as f64;
            let combined_bps = rx_bps + tx_bps;

            total_rx_bps += rx_bps;
            total_tx_bps += tx_bps;

            let history = self.iface_histories.entry(name.clone()).or_default();
            history.push(rx_bps, tx_bps);

            let consistency = consistency_score(&history.combined);
            let heavy_consistent =
                combined_bps >= HEAVY_RATE_BPS && consistency >= CONSISTENCY_FLOOR;

            interfaces.push(InterfaceStats {
                name: name.clone(),
                rx_bps,
                tx_bps,
                combined_bps,
                total_rx: data.total_received(),
                total_tx: data.total_transmitted(),
                consistency,
                heavy_consistent,
                rx_history: history.rx.clone(),
                tx_history: history.tx.clone(),
                combined_history: history.combined.clone(),
            });
        }

        sort_interfaces(&mut interfaces);

        let mut system = System::new();
        system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        let process_count = system.processes().len();

        NetworkSnapshot {
            interfaces,
            rx_history: Vec::new(),
            tx_history: Vec::new(),
            combined_history: Vec::new(),
            total_rx_bps,
            total_tx_bps,
            process_count,
            connection_count,
            sample_tick: 0,
        }
    }
}

pub fn push_history(snapshot: &mut NetworkSnapshot, previous: &NetworkSnapshot) {
    snapshot.rx_history = previous.rx_history.clone();
    snapshot.tx_history = previous.tx_history.clone();
    snapshot.combined_history = previous.combined_history.clone();
    snapshot.sample_tick = previous.sample_tick.saturating_add(1);

    snapshot.rx_history.push(snapshot.total_rx_bps);
    snapshot.tx_history.push(snapshot.total_tx_bps);
    snapshot
        .combined_history
        .push(snapshot.total_rx_bps + snapshot.total_tx_bps);

    for series in [
        &mut snapshot.rx_history,
        &mut snapshot.tx_history,
        &mut snapshot.combined_history,
    ] {
        if series.len() > HISTORY_LEN {
            series.remove(0);
        }
    }
}

fn consistency_score(history: &[f64]) -> f64 {
    if history.len() < 4 {
        return 0.0;
    }

    let avg = history.iter().sum::<f64>() / history.len() as f64;
    if avg <= 0.0 {
        return 0.0;
    }

    let variance =
        history.iter().map(|v| (v - avg).powi(2)).sum::<f64>() / history.len() as f64;
    let cv = (variance.sqrt() / avg).min(1.0);
    1.0 - cv
}

/// Heavy, steady interfaces float to the top — no visible pin chrome.
fn sort_interfaces(interfaces: &mut [InterfaceStats]) {
    interfaces.sort_by(|a, b| {
        match (a.heavy_consistent, b.heavy_consistent) {
            (true, false) => return std::cmp::Ordering::Less,
            (false, true) => return std::cmp::Ordering::Greater,
            _ => {}
        }

        let a_score = a.combined_bps * (0.35 + a.consistency);
        let b_score = b.combined_bps * (0.35 + b.consistency);
        b_score
            .partial_cmp(&a_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.name.cmp(&b.name))
    });
}

pub const POLL_INTERVAL: Duration = Duration::from_secs(1);

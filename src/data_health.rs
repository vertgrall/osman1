//! User-facing data collection health — when adapters or subprocess collectors fail.

use crate::detail::TrafficSnapshot;
use crate::network::NetworkSnapshot;

/// Result of nettop / lsof collection attempts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DataHealth {
    pub nettop_tcp_ok: bool,
    pub nettop_udp_ok: bool,
    pub lsof_ok: bool,
    pub nettop_rows: usize,
    pub lsof_rows: usize,
}

impl DataHealth {
    pub fn nettop_ok(&self) -> bool {
        self.nettop_tcp_ok || self.nettop_udp_ok
    }

    pub fn from_collect(nettop_tcp_ok: bool, nettop_udp_ok: bool, lsof_ok: bool) -> Self {
        Self {
            nettop_tcp_ok,
            nettop_udp_ok,
            lsof_ok,
            ..Self::default()
        }
    }

    /// Highest-priority banner for Overview (connections / subprocess issues).
    pub fn overview_banner(&self) -> Option<&'static str> {
        if !self.nettop_ok() && !self.lsof_ok {
            return Some(
                "Connection details unavailable. Install Xcode Command Line Tools: xcode-select --install",
            );
        }
        if !self.nettop_ok() {
            return Some(
                "Connection details unavailable (nettop failed). Adapter totals still update.",
            );
        }
        if !self.lsof_ok {
            return Some("Listener list may be incomplete (lsof unavailable).");
        }
        None
    }

    /// Copy for empty adapter table / first-sample state.
    pub fn adapter_empty_message(snapshot: &NetworkSnapshot) -> &'static str {
        if snapshot.sample_tick == 0 {
            "Waiting for first adapter sample…"
        } else {
            "No active network adapters detected (loopback is hidden)."
        }
    }

    /// Combined Overview status: adapter state wins over subprocess warnings.
    pub fn overview_status(
        network: &NetworkSnapshot,
        traffic: &TrafficSnapshot,
    ) -> Option<String> {
        if network.interfaces.is_empty() {
            return Some(Self::adapter_empty_message(network).into());
        }
        traffic.health.overview_banner().map(str::to_string)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::NetworkSnapshot;

    #[test]
    fn banner_none_when_healthy() {
        let health = DataHealth {
            nettop_tcp_ok: true,
            nettop_udp_ok: true,
            lsof_ok: true,
            ..Default::default()
        };
        assert_eq!(health.overview_banner(), None);
    }

    #[test]
    fn banner_nettop_and_lsof_failed() {
        let health = DataHealth::default();
        assert!(health.overview_banner().unwrap().contains("xcode-select"));
    }

    #[test]
    fn banner_nettop_only_failed() {
        let health = DataHealth {
            lsof_ok: true,
            ..Default::default()
        };
        assert!(health
            .overview_banner()
            .unwrap()
            .contains("nettop failed"));
    }

    #[test]
    fn banner_lsof_only_failed() {
        let health = DataHealth {
            nettop_tcp_ok: true,
            nettop_udp_ok: true,
            lsof_ok: false,
            ..Default::default()
        };
        assert!(health.overview_banner().unwrap().contains("lsof"));
    }

    #[test]
    fn banner_waiting_first_sample() {
        let snap = NetworkSnapshot::default();
        assert_eq!(
            DataHealth::adapter_empty_message(&snap),
            "Waiting for first adapter sample…"
        );
    }

    #[test]
    fn banner_no_adapters_after_tick() {
        let snap = NetworkSnapshot {
            sample_tick: 3,
            ..Default::default()
        };
        assert!(DataHealth::adapter_empty_message(&snap).contains("No active network adapters"));
    }

    #[test]
    fn overview_status_prefers_empty_adapters() {
        let network = NetworkSnapshot {
            sample_tick: 2,
            ..Default::default()
        };
        let traffic = TrafficSnapshot {
            health: DataHealth::default(),
            ..Default::default()
        };
        let msg = DataHealth::overview_status(&network, &traffic).unwrap();
        assert!(msg.contains("No active network adapters"));
    }
}

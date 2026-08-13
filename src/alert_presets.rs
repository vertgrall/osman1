//! Built-in alert presets — common monitoring "plays" as one-click rule bundles.

use crate::alerts::StoredAlertRule;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AlertPreset {
    pub id: &'static str,
    pub label: &'static str,
    pub blurb: &'static str,
    rules: [StoredAlertRule; 5],
}

impl AlertPreset {
    pub const ALL: [Self; 6] = [
        Self::BALANCED,
        Self::SECURITY_WATCH,
        Self::QUIET_MONITOR,
        Self::DEVELOPER,
        Self::METERED,
        Self::HEAVY_DOWNLOAD,
    ];

    pub const BALANCED: Self = Self {
        id: "balanced",
        label: "Balanced",
        blurb: "Default mix — bandwidth + security, moderate noise.",
        rules: preset_rules(
            (true, 10_000_000.0, 5),
            (true, 5_000_000.0, 8),
            (true, 0.0, 1),
            (true, 0.0, 1),
            (true, 0.0, 15),
        ),
    };

    pub const SECURITY_WATCH: Self = Self {
        id: "security_watch",
        label: "Security Watch",
        blurb: "Focus on new public connections, listeners, and fan-out.",
        rules: preset_rules(
            (false, 10_000_000.0, 5),
            (false, 5_000_000.0, 8),
            (true, 0.0, 1),
            (true, 0.0, 1),
            (true, 0.0, 10),
        ),
    };

    pub const QUIET_MONITOR: Self = Self {
        id: "quiet_monitor",
        label: "Quiet Monitor",
        blurb: "Only critical surface area — wildcard listeners and fan-out.",
        rules: preset_rules(
            (false, 10_000_000.0, 5),
            (false, 5_000_000.0, 8),
            (false, 0.0, 1),
            (true, 0.0, 1),
            (true, 0.0, 15),
        ),
    };

    pub const DEVELOPER: Self = Self {
        id: "developer",
        label: "Developer",
        blurb: "Tolerate Docker/npm noise — high bandwidth bar, no public-outbound spam.",
        rules: preset_rules(
            (true, 50_000_000.0, 10),
            (true, 25_000_000.0, 10),
            (false, 0.0, 1),
            (true, 0.0, 1),
            (true, 0.0, 25),
        ),
    };

    pub const METERED: Self = Self {
        id: "metered",
        label: "Metered / Hotspot",
        blurb: "Low bandwidth ceilings for cellular or tethered links.",
        rules: preset_rules(
            (true, 2_000_000.0, 3),
            (true, 1_000_000.0, 5),
            (true, 0.0, 1),
            (true, 0.0, 1),
            (true, 0.0, 12),
        ),
    };

    pub const HEAVY_DOWNLOAD: Self = Self {
        id: "heavy_download",
        label: "Heavy Download",
        blurb: "Large transfers OK — only flag extreme spikes and listeners.",
        rules: preset_rules(
            (true, 50_000_000.0, 15),
            (true, 30_000_000.0, 15),
            (false, 0.0, 1),
            (true, 0.0, 1),
            (true, 0.0, 30),
        ),
    };

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|preset| preset.id == id)
    }

    pub fn rules(&self) -> &[StoredAlertRule] {
        &self.rules
    }
}

pub fn default_preset_id() -> &'static str {
    AlertPreset::BALANCED.id
}

const fn preset_rules(
    r1: (bool, f64, u32),
    r2: (bool, f64, u32),
    r3: (bool, f64, u32),
    r4: (bool, f64, u32),
    r5: (bool, f64, u32),
) -> [StoredAlertRule; 5] {
    [
        StoredAlertRule {
            id: 1,
            enabled: r1.0,
            threshold_bps: r1.1,
            sustained_secs: r1.2,
        },
        StoredAlertRule {
            id: 2,
            enabled: r2.0,
            threshold_bps: r2.1,
            sustained_secs: r2.2,
        },
        StoredAlertRule {
            id: 3,
            enabled: r3.0,
            threshold_bps: r3.1,
            sustained_secs: r3.2,
        },
        StoredAlertRule {
            id: 4,
            enabled: r4.0,
            threshold_bps: r4.1,
            sustained_secs: r4.2,
        },
        StoredAlertRule {
            id: 5,
            enabled: r5.0,
            threshold_bps: r5.1,
            sustained_secs: r5.2,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_presets_define_five_rules() {
        for preset in AlertPreset::ALL {
            assert_eq!(preset.rules().len(), 5);
            for (i, rule) in preset.rules().iter().enumerate() {
                assert_eq!(rule.id, (i + 1) as u32);
            }
        }
    }

    #[test]
    fn security_watch_disables_bandwidth_rules() {
        let rules = AlertPreset::SECURITY_WATCH.rules();
        assert!(!rules[0].enabled);
        assert!(!rules[1].enabled);
        assert!(rules[2].enabled);
    }

    #[test]
    fn developer_disables_public_outbound() {
        let rules = AlertPreset::DEVELOPER.rules();
        assert!(!rules[2].enabled);
    }

    #[test]
    fn from_id_resolves_known_presets() {
        assert_eq!(
            AlertPreset::from_id("metered"),
            Some(AlertPreset::METERED)
        );
        assert_eq!(AlertPreset::from_id("unknown"), None);
    }
}

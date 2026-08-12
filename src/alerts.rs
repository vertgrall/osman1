use std::collections::{HashMap, HashSet, VecDeque};
use std::process::Command;
use std::time::{Duration, Instant};

use freya::prelude::*;

use crate::detail::ConnectionDetail;
use crate::network::NetworkSnapshot;
use crate::parse::{is_wildcard_host, Direction, SocketRole};
use crate::theme::{format_rate, Palette};

const LOG_CAP: usize = 40;
const FANOUT_JUMP: usize = 15;
const CONNECTION_COOLDOWN_SECS: u64 = 90;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Clone, Debug)]
pub struct AlertEvent {
    pub at: Instant,
    pub message: String,
    pub severity: AlertSeverity,
}

#[derive(Clone, Debug)]
pub struct AlertRule {
    pub id: u32,
    pub label: String,
    pub threshold_bps: f64,
    pub sustained_secs: u32,
    pub enabled: bool,
}

impl AlertRule {
    fn defaults() -> Vec<Self> {
        vec![
            Self {
                id: 1,
                label: "Total bandwidth spike".into(),
                threshold_bps: 10_000_000.0,
                sustained_secs: 5,
                enabled: true,
            },
            Self {
                id: 2,
                label: "Adapter heavy load".into(),
                threshold_bps: 5_000_000.0,
                sustained_secs: 8,
                enabled: true,
            },
            Self {
                id: 3,
                label: "New outbound public connection".into(),
                threshold_bps: 0.0,
                sustained_secs: 1,
                enabled: true,
            },
            Self {
                id: 4,
                label: "Wildcard listener".into(),
                threshold_bps: 0.0,
                sustained_secs: 1,
                enabled: true,
            },
            Self {
                id: 5,
                label: "Connection fan-out spike".into(),
                threshold_bps: 0.0,
                sustained_secs: 1,
                enabled: true,
            },
        ]
    }
}

impl Default for AlertEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct AlertEngine {
    rules: Vec<AlertRule>,
    events: VecDeque<AlertEvent>,
    breach_ticks: HashMap<u32, u32>,
    last_notify: HashMap<u32, Instant>,
    seen_connection_ids: HashSet<u64>,
    notified_listeners: HashSet<(u32, Option<u16>)>,
    prev_connection_count: usize,
}

impl AlertEngine {
    pub fn new() -> Self {
        Self {
            rules: AlertRule::defaults(),
            events: VecDeque::new(),
            breach_ticks: HashMap::new(),
            last_notify: HashMap::new(),
            seen_connection_ids: HashSet::new(),
            notified_listeners: HashSet::new(),
            prev_connection_count: 0,
        }
    }

    pub fn events(&self) -> &VecDeque<AlertEvent> {
        &self.events
    }

    pub fn rules(&self) -> &[AlertRule] {
        &self.rules
    }

    pub fn toggle_rule(&mut self, id: u32) {
        if let Some(rule) = self.rules.iter_mut().find(|r| r.id == id) {
            rule.enabled = !rule.enabled;
        }
    }

    pub fn evaluate(&mut self, snapshot: &NetworkSnapshot, connections: &[ConnectionDetail]) {
        let total = snapshot.total_rx_bps + snapshot.total_tx_bps;
        self.check_rule(1, total, |t| {
            format!(
                "Total traffic exceeded {} (now {})",
                format_rate(t),
                format_rate(total)
            )
        });

        let peak_adapter = snapshot
            .interfaces
            .iter()
            .map(|i| i.combined_bps)
            .fold(0.0_f64, f64::max);
        self.check_rule(2, peak_adapter, |t| {
            format!(
                "Adapter load exceeded {} (peak {})",
                format_rate(t),
                format_rate(peak_adapter)
            )
        });

        self.evaluate_connection_rules(connections);
        self.prev_connection_count = connections.len();
    }

    fn evaluate_connection_rules(&mut self, connections: &[ConnectionDetail]) {
        if self.rule_enabled(3) {
            for conn in connections {
                if conn.direction != Direction::Outbound {
                    continue;
                }
                if conn.remote_is_private || conn.remote_is_loopback {
                    continue;
                }
                if self.seen_connection_ids.contains(&conn.id.0) {
                    continue;
                }
                let msg = format!(
                    "{} → {} ({})",
                    conn.process_name,
                    conn.remote_label(),
                    conn.transport
                );
                self.notify_connection_rule(3, msg, AlertSeverity::Warning);
            }
        }

        if self.rule_enabled(4) {
            for conn in connections {
                if conn.role != SocketRole::Listener {
                    continue;
                }
                if !is_wildcard_host(&conn.local_host) {
                    continue;
                }
                let key = (conn.pid, conn.local_port);
                if self.notified_listeners.contains(&key) {
                    continue;
                }
                let port = conn
                    .local_port
                    .map(|p| p.to_string())
                    .unwrap_or_else(|| "?".into());
                let msg = format!(
                    "{} listening on *:{} ({})",
                    conn.process_name, port, conn.transport
                );
                self.notify_connection_rule(4, msg, AlertSeverity::Info);
                self.notified_listeners.insert(key);
            }
        }

        if self.rule_enabled(5) && self.prev_connection_count > 0 {
            let jump = connections.len().saturating_sub(self.prev_connection_count);
            if jump >= FANOUT_JUMP {
                let msg = format!(
                    "Connections jumped by {} (now {})",
                    jump,
                    connections.len()
                );
                self.notify_connection_rule(5, msg, AlertSeverity::Warning);
            }
        }

        self.seen_connection_ids = connections.iter().map(|c| c.id.0).collect();
    }

    fn rule_enabled(&self, id: u32) -> bool {
        self.rules.iter().any(|r| r.id == id && r.enabled)
    }

    fn notify_connection_rule(&mut self, id: u32, message: String, severity: AlertSeverity) {
        let cooldown = self
            .last_notify
            .get(&id)
            .is_none_or(|t| t.elapsed() > Duration::from_secs(CONNECTION_COOLDOWN_SECS));

        if cooldown {
            notify_macos("Osman Alert", &message);
            self.last_notify.insert(id, Instant::now());
            self.push_event(message, severity);
        }
    }

    fn check_rule<F>(&mut self, id: u32, value: f64, message: F)
    where
        F: FnOnce(f64) -> String,
    {
        let Some(rule) = self.rules.iter().find(|r| r.id == id && r.enabled).cloned() else {
            self.breach_ticks.remove(&id);
            return;
        };

        if value >= rule.threshold_bps {
            let ticks = self.breach_ticks.entry(id).or_insert(0);
            *ticks += 1;

            if *ticks >= rule.sustained_secs {
                let cooldown = self
                    .last_notify
                    .get(&id)
                    .is_none_or(|t| t.elapsed() > Duration::from_secs(60));

                if cooldown {
                    let msg = message(rule.threshold_bps);
                    notify_macos("Osman Alert", &msg);
                    self.last_notify.insert(id, Instant::now());
                    self.push_event(msg, AlertSeverity::Warning);
                }
                self.breach_ticks.insert(id, 0);
            }
        } else {
            self.breach_ticks.remove(&id);
        }
    }

    fn push_event(&mut self, message: String, severity: AlertSeverity) {
        self.events.push_front(AlertEvent {
            at: Instant::now(),
            message,
            severity,
        });
        while self.events.len() > LOG_CAP {
            self.events.pop_back();
        }
    }
}

fn notify_macos(title: &str, body: &str) {
    let title = escape_applescript(title);
    let body = escape_applescript(body);
    let script = format!("display notification \"{body}\" with title \"{title}\"");
    let _ = Command::new("osascript").args(["-e", &script]).spawn();
}

fn escape_applescript(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

pub fn alerts_screen(engine: &AlertEngine, palette: Palette) -> Element {
    let rules: Vec<Element> = engine
        .rules()
        .iter()
        .map(|rule| alert_rule_row(rule, palette))
        .collect();

    let events: Vec<Element> = if engine.events().is_empty() {
        vec![label()
            .text("No alerts yet — rules monitor traffic every second.")
            .font_size(11.)
            .color(palette.muted)
            .into()]
    } else {
        engine
            .events()
            .iter()
            .take(12)
            .map(|ev| alert_event_row(ev, palette))
            .collect()
    };

    rect()
        .vertical()
        .expanded()
        .spacing(12.)
        .background(palette.panel)
        .corner_radius(12.)
        .border(palette.border())
        .padding(Gaps::new_all(12.))
        .child(
            label()
                .text("Alert rules")
                .font_size(14.)
                .font_weight(FontWeight::BOLD)
                .color(palette.text),
        )
        .child(
            rect()
                .vertical()
                .spacing(6.)
                .children(rules),
        )
        .child(
            label()
                .text("Recent alerts")
                .font_size(14.)
                .font_weight(FontWeight::BOLD)
                .color(palette.text),
        )
        .child(
            rect()
                .vertical()
                .spacing(4.)
                .children(events),
        )
        .into()
}

fn alert_rule_row(rule: &AlertRule, palette: Palette) -> Element {
    let status = if rule.enabled { "On" } else { "Off" };
    let status_color = if rule.enabled {
        palette.send
    } else {
        palette.muted
    };

    let detail = if rule.id <= 2 {
        format!(
            "Threshold {} · sustained {}s",
            format_rate(rule.threshold_bps),
            rule.sustained_secs
        )
    } else {
        "Event-driven".into()
    };

    rect()
        .horizontal()
        .width(Size::fill())
        .padding(Gaps::new_all(8.))
        .background(palette.bg)
        .corner_radius(8.)
        .border(palette.border())
        .child(
            rect()
                .vertical()
                .spacing(2.)
                .child(
                    label()
                        .text(rule.label.clone())
                        .font_size(12.)
                        .font_weight(FontWeight::BOLD)
                        .color(palette.text),
                )
                .child(
                    label()
                        .text(detail)
                        .font_size(10.)
                        .color(palette.muted),
                ),
        )
        .child(
            label()
                .text(status)
                .font_size(11.)
                .font_weight(FontWeight::BOLD)
                .color(status_color),
        )
        .into()
}

fn alert_event_row(ev: &AlertEvent, palette: Palette) -> Element {
    let color = match ev.severity {
        AlertSeverity::Info => palette.receive,
        AlertSeverity::Warning => palette.total,
        AlertSeverity::Critical => Color::from_rgb(196, 92, 72),
    };
    let ago = format!("{}s ago", ev.at.elapsed().as_secs());

    rect()
        .horizontal()
        .width(Size::fill())
        .padding(Gaps::new_all(8.))
        .background(palette.bg)
        .corner_radius(8.)
        .spacing(8.)
        .child(
            rect()
                .width(Size::px(6.))
                .height(Size::px(6.))
                .background(color)
                .corner_radius(3.),
        )
        .child(
            label()
                .text(ev.message.clone())
                .font_size(11.)
                .color(palette.text),
        )
        .child(
            label()
                .text(ago)
                .font_size(10.)
                .color(palette.muted),
        )
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::{DataSource, HostPort, SocketRole};

    fn sample_conn(id: u64, remote: &str, private: bool) -> ConnectionDetail {
        let local = HostPort {
            host: "10.0.0.2".into(),
            port: Some(44_000),
        };
        let remote_hp = HostPort {
            host: remote.into(),
            port: Some(443),
        };
        ConnectionDetail {
            id: crate::parse::ConnectionId(id),
            process_name: "curl".into(),
            pid: 99,
            interface: "en0".into(),
            protocol: "tcp4".into(),
            transport: "TCP".into(),
            endpoint: String::new(),
            state: "Established".into(),
            local_host: local.host.clone(),
            local_port: local.port,
            remote_host: remote_hp.host.clone(),
            remote_port: remote_hp.port,
            role: SocketRole::Established,
            direction: Direction::Outbound,
            remote_is_private: private,
            remote_is_loopback: false,
            rx_bytes: 0,
            tx_bytes: 0,
            source: DataSource::Nettop,
        }
    }

    #[test]
    fn alerts_on_new_public_outbound() {
        let mut engine = AlertEngine::new();
        let snap = NetworkSnapshot::default();
        let conn = sample_conn(42, "104.18.18.125", false);

        engine.evaluate(&snap, std::slice::from_ref(&conn));
        assert!(!engine.events().is_empty());
        assert!(engine.events()[0].message.contains("104.18.18.125"));
    }
}

use std::collections::{HashMap, HashSet, VecDeque};
use std::process::Command;
use std::time::{Duration, Instant};

use freya::prelude::*;
use serde::{Deserialize, Serialize};

use crate::alert_presets::AlertPreset;
use crate::detail::ConnectionDetail;
use crate::network::NetworkSnapshot;
use crate::parse::{is_wildcard_host, Direction, SocketRole};
use crate::preferences;
use crate::theme::{format_rate, Palette};

const LOG_CAP: usize = 40;
const CONNECTION_COOLDOWN_SECS: u64 = 90;
pub const RECENT_ALERT_WINDOW: Duration = Duration::from_secs(120);

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

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct StoredAlertRule {
    pub id: u32,
    pub threshold_bps: f64,
    pub sustained_secs: u32,
    pub enabled: bool,
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
                sustained_secs: 15,
                enabled: true,
            },
        ]
    }

    fn to_stored(&self) -> StoredAlertRule {
        StoredAlertRule {
            id: self.id,
            threshold_bps: self.threshold_bps,
            sustained_secs: self.sustained_secs,
            enabled: self.enabled,
        }
    }
}

impl Default for AlertEngine {
    fn default() -> Self {
        Self::load()
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
    pub fn load() -> Self {
        Self::from_stored_rules(&preferences::get().alert_rules)
    }

    pub fn from_stored_rules(stored: &[StoredAlertRule]) -> Self {
        let rules = if stored.is_empty() {
            AlertRule::defaults()
        } else {
            Self::rules_from_stored(stored)
        };
        Self {
            rules,
            events: VecDeque::new(),
            breach_ticks: HashMap::new(),
            last_notify: HashMap::new(),
            seen_connection_ids: HashSet::new(),
            notified_listeners: HashSet::new(),
            prev_connection_count: 0,
        }
    }

    pub fn new() -> Self {
        Self::load()
    }

    pub fn events(&self) -> &VecDeque<AlertEvent> {
        &self.events
    }

    pub fn rules(&self) -> &[AlertRule] {
        &self.rules
    }

    pub fn recent_event_count(&self, within: Duration) -> usize {
        self.events
            .iter()
            .filter(|ev| ev.at.elapsed() < within)
            .count()
    }

    pub fn stored_rules(&self) -> Vec<StoredAlertRule> {
        self.rules.iter().map(AlertRule::to_stored).collect()
    }

    pub fn toggle_rule(&mut self, id: u32) {
        if let Some(rule) = self.rules.iter_mut().find(|r| r.id == id) {
            rule.enabled = !rule.enabled;
            self.persist_rules();
            self.mark_preset_custom();
        }
    }

    pub fn update_rule(&mut self, id: u32, threshold_bps: f64, sustained_secs: u32) {
        if let Some(rule) = self.rules.iter_mut().find(|r| r.id == id) {
            rule.threshold_bps = threshold_bps.max(0.0);
            rule.sustained_secs = sustained_secs.max(1);
            self.persist_rules();
            self.mark_preset_custom();
        }
    }

    pub fn bump_threshold(&mut self, id: u32, delta_bps: f64) {
        if let Some(rule) = self.rules.iter().find(|r| r.id == id).cloned() {
            let next = (rule.threshold_bps + delta_bps).max(0.0);
            self.update_rule(id, next, rule.sustained_secs);
        }
    }

    pub fn bump_sustained(&mut self, id: u32, delta: i32) {
        if let Some(rule) = self.rules.iter().find(|r| r.id == id).cloned() {
            let next = (rule.sustained_secs as i32 + delta).max(1) as u32;
            self.update_rule(id, rule.threshold_bps, next);
        }
    }

    pub fn apply_preset(&mut self, preset: AlertPreset) {
        self.rules = Self::rules_from_stored(preset.rules());
        let stored = self.stored_rules();
        let preset_id = preset.id;
        let _ = preferences::with_store(|store| {
            store.prefs.alert_rules = stored;
            store.set_alert_preset(preset_id)
        });
    }

    fn mark_preset_custom(&self) {
        if preferences::get().alert_preset != "custom" {
            let _ = preferences::with_store(|store| store.set_alert_preset("custom"));
        }
    }

    fn rules_from_stored(stored: &[StoredAlertRule]) -> Vec<AlertRule> {
        let mut rules = AlertRule::defaults();
        for saved in stored {
            if let Some(rule) = rules.iter_mut().find(|r| r.id == saved.id) {
                rule.enabled = saved.enabled;
                rule.threshold_bps = saved.threshold_bps.max(0.0);
                rule.sustained_secs = saved.sustained_secs.max(1);
            }
        }
        rules
    }

    pub fn clear_events(&mut self) {
        self.events.clear();
    }

    fn persist_rules(&self) {
        let stored = self.stored_rules();
        let _ = preferences::with_store(|store| store.set_alert_rules(stored));
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
                self.notify_connection_rule(4, msg, AlertSeverity::Critical);
                self.notified_listeners.insert(key);
            }
        }

        if self.rule_enabled(5) && self.prev_connection_count > 0 {
            let jump_threshold = self
                .rules
                .iter()
                .find(|r| r.id == 5)
                .map(|r| r.sustained_secs as usize)
                .unwrap_or(15);
            let jump = connections.len().saturating_sub(self.prev_connection_count);
            if jump >= jump_threshold {
                let msg = format!(
                    "Connections jumped by {} (now {})",
                    jump,
                    connections.len()
                );
                self.notify_connection_rule(5, msg, AlertSeverity::Critical);
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

pub fn format_alert_age(at: Instant) -> String {
    let secs = at.elapsed().as_secs();
    if secs < 5 {
        "just now".into()
    } else if secs < 60 {
        format!("{secs}s ago")
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else {
        format!("{}h ago", secs / 3600)
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

pub fn active_alert_preset_id() -> String {
    preferences::get().alert_preset
}

pub fn alerts_screen(mut alert_engine: State<AlertEngine>, palette: Palette) -> Element {
    let engine = alert_engine.read();
    let active_preset = active_alert_preset_id();
    let rules: Vec<Element> = engine
        .rules()
        .iter()
        .map(|rule| alert_rule_row(rule, palette, alert_engine))
        .collect();

    let events: Vec<Element> = if engine.events().is_empty() {
        vec![label()
            .text("No alerts yet — rules monitor traffic every poll.")
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
                .text("Alert preset")
                .font_size(14.)
                .font_weight(FontWeight::BOLD)
                .color(palette.text),
        )
        .child(
            label()
                .text(if active_preset == "custom" {
                    "Active: Custom — tap a preset below to replace your tweaks."
                } else {
                    "One-click plays for common monitoring scenarios."
                })
                .font_size(11.)
                .color(palette.muted),
        )
        .child(alert_preset_picker(&active_preset, palette, alert_engine))
        .child(
            label()
                .text("Alert rules")
                .font_size(14.)
                .font_weight(FontWeight::BOLD)
                .color(palette.text),
        )
        .child(
            label()
                .text("Click a rule to enable or disable. Bandwidth rules support ± controls.")
                .font_size(11.)
                .color(palette.muted),
        )
        .child(
            rect()
                .vertical()
                .spacing(6.)
                .children(rules),
        )
        .child(
            rect()
                .horizontal()
                .width(Size::fill())
                .child(
                    label()
                        .text("Recent alerts")
                        .font_size(14.)
                        .font_weight(FontWeight::BOLD)
                        .color(palette.text),
                )
                .child(
                    rect()
                        .padding(Gaps::new(6., 10., 6., 10.))
                        .background(palette.bg)
                        .corner_radius(6.)
                        .border(palette.border())
                        .on_mouse_up(move |_| alert_engine.write_unchecked().clear_events())
                        .child(
                            label()
                                .text("Clear log")
                                .font_size(10.)
                                .color(palette.muted),
                        ),
                ),
        )
        .child(
            rect()
                .vertical()
                .spacing(4.)
                .children(events),
        )
        .into()
}

fn alert_preset_picker(
    active_id: &str,
    palette: Palette,
    mut alert_engine: State<AlertEngine>,
) -> Element {
    rect()
        .vertical()
        .spacing(6.)
        .children(
            AlertPreset::ALL
                .iter()
                .map(|preset| preset_card(*preset, active_id, palette, alert_engine))
                .collect::<Vec<_>>(),
        )
        .into()
}

fn preset_card(
    preset: AlertPreset,
    active_id: &str,
    palette: Palette,
    mut alert_engine: State<AlertEngine>,
) -> Element {
    let is_active = preset.id == active_id;
    let bg = if is_active {
        palette.selected_bg()
    } else {
        palette.bg
    };
    let border = if is_active {
        Border::new().fill(palette.accent).width(1.5)
    } else {
        palette.border()
    };

    rect()
        .horizontal()
        .width(Size::fill())
        .padding(Gaps::new(8., 10., 8., 10.))
        .background(bg)
        .corner_radius(8.)
        .border(border)
        .spacing(10.)
        .on_mouse_up(move |_| alert_engine.write_unchecked().apply_preset(preset))
        .child(
            rect()
                .vertical()
                .spacing(2.)
                .child(
                    label()
                        .text(preset.label)
                        .font_size(12.)
                        .font_weight(if is_active {
                            FontWeight::BOLD
                        } else {
                            FontWeight::NORMAL
                        })
                        .color(if is_active {
                            palette.text
                        } else {
                            palette.muted
                        }),
                )
                .child(
                    label()
                        .text(preset.blurb)
                        .font_size(10.)
                        .color(palette.muted),
                ),
        )
        .child(
            label()
                .text(if is_active {
                    "Active"
                } else {
                    "Apply"
                })
                .font_size(10.)
                .font_weight(FontWeight::BOLD)
                .color(if is_active {
                    palette.receive
                } else {
                    palette.muted
                }),
        )
        .into()
}

fn alert_rule_row(rule: &AlertRule, palette: Palette, mut alert_engine: State<AlertEngine>) -> Element {
    let enabled = rule.enabled;
    let rule_id = rule.id;
    let text_color = if enabled {
        palette.text
    } else {
        palette.muted
    };
    let row_bg = if enabled {
        palette.bg
    } else {
        Color::from_argb(18, palette.text.r(), palette.text.g(), palette.text.b())
    };

    let mut row = rect()
        .horizontal()
        .width(Size::fill())
        .padding(Gaps::new_all(8.))
        .background(row_bg)
        .corner_radius(8.)
        .border(palette.border())
        .spacing(8.)
        .on_mouse_up(move |_| alert_engine.write_unchecked().toggle_rule(rule_id))
        .child(
            label()
                .text(if enabled { "✓" } else { "○" })
                .font_size(12.)
                .font_weight(FontWeight::BOLD)
                .color(if enabled {
                    palette.receive
                } else {
                    palette.muted
                }),
        )
        .child(
            rect()
                .vertical()
                .spacing(2.)
                .child(
                    label()
                        .text(rule.label.clone())
                        .font_size(12.)
                        .font_weight(FontWeight::BOLD)
                        .color(text_color),
                )
                .child(
                    label()
                        .text(rule_detail_line(rule))
                        .font_size(10.)
                        .color(palette.muted),
                ),
        );

    if rule.id <= 2 {
        row = row.child(threshold_stepper(rule, palette, alert_engine));
    } else if rule.id == 5 {
        row = row.child(fanout_stepper(rule, palette, alert_engine));
    }

    row.into()
}

fn rule_detail_line(rule: &AlertRule) -> String {
    match rule.id {
        1 | 2 => format!(
            "Threshold {} · sustained {}s",
            format_rate(rule.threshold_bps),
            rule.sustained_secs
        ),
        5 => format!("Jump ≥ {} connections in one poll", rule.sustained_secs),
        _ => "Event-driven".into(),
    }
}

fn threshold_stepper(rule: &AlertRule, palette: Palette, mut alert_engine: State<AlertEngine>) -> Element {
    let rule_id = rule.id;
    rect()
        .horizontal()
        .spacing(4.)
        .child(stepper_button("−", palette, move || {
            alert_engine.write_unchecked().bump_threshold(rule_id, -1_000_000.0);
        }))
        .child(
            label()
                .text(format_rate(rule.threshold_bps))
                .font_size(10.)
                .font_weight(FontWeight::BOLD)
                .color(palette.text),
        )
        .child(stepper_button("+", palette, move || {
            alert_engine.write_unchecked().bump_threshold(rule_id, 1_000_000.0);
        }))
        .child(stepper_button("s−", palette, move || {
            alert_engine.write_unchecked().bump_sustained(rule_id, -1);
        }))
        .child(
            label()
                .text(format!("{}s", rule.sustained_secs))
                .font_size(10.)
                .color(palette.muted),
        )
        .child(stepper_button("s+", palette, move || {
            alert_engine.write_unchecked().bump_sustained(rule_id, 1);
        }))
        .into()
}

fn fanout_stepper(rule: &AlertRule, palette: Palette, mut alert_engine: State<AlertEngine>) -> Element {
    let rule_id = rule.id;
    rect()
        .horizontal()
        .spacing(4.)
        .child(stepper_button("−", palette, move || {
            alert_engine.write_unchecked().bump_sustained(rule_id, -1);
        }))
        .child(
            label()
                .text(format!("≥{}", rule.sustained_secs))
                .font_size(10.)
                .font_weight(FontWeight::BOLD)
                .color(palette.text),
        )
        .child(stepper_button("+", palette, move || {
            alert_engine.write_unchecked().bump_sustained(rule_id, 1);
        }))
        .into()
}

fn stepper_button(
    caption: &'static str,
    palette: Palette,
    on_press: impl FnMut() + 'static,
) -> Element {
    let mut handler = on_press;
    rect()
        .padding(Gaps::new(4., 8., 4., 8.))
        .background(palette.panel)
        .corner_radius(6.)
        .border(palette.border())
        .on_mouse_up(move |_| handler())
        .child(
            label()
                .text(caption)
                .font_size(10.)
                .font_weight(FontWeight::BOLD)
                .color(palette.text),
        )
        .into()
}

fn alert_event_row(ev: &AlertEvent, palette: Palette) -> Element {
    let color = match ev.severity {
        AlertSeverity::Info => palette.receive,
        AlertSeverity::Warning => palette.total,
        AlertSeverity::Critical => Color::from_rgb(196, 92, 72),
    };
    let ago = format_alert_age(ev.at);

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

    #[test]
    fn toggle_rule_flips_enabled() {
        let mut engine = AlertEngine::from_stored_rules(&[]);
        let before = engine
            .rules()
            .iter()
            .find(|r| r.id == 1)
            .expect("rule 1")
            .enabled;
        engine.toggle_rule(1);
        let after = engine
            .rules()
            .iter()
            .find(|r| r.id == 1)
            .expect("rule 1")
            .enabled;
        assert_ne!(before, after);
    }

    #[test]
    fn update_rule_changes_threshold() {
        let mut engine = AlertEngine::new();
        engine.update_rule(1, 2_000_000.0, 10);
        assert_eq!(engine.rules()[0].threshold_bps, 2_000_000.0);
        assert_eq!(engine.rules()[0].sustained_secs, 10);
    }

    #[test]
    fn clear_events_empties_log() {
        let mut engine = AlertEngine::new();
        let snap = NetworkSnapshot::default();
        let conn = sample_conn(7, "104.18.18.125", false);
        engine.evaluate(&snap, std::slice::from_ref(&conn));
        assert!(!engine.events().is_empty());
        engine.clear_events();
        assert!(engine.events().is_empty());
    }

    #[test]
    fn stored_rules_round_trip_defaults() {
        let engine = AlertEngine::new();
        let stored = engine.stored_rules();
        let restored = AlertEngine::from_stored_rules(&stored);
        assert_eq!(engine.rules().len(), restored.rules().len());
        assert_eq!(engine.rules()[0].id, restored.rules()[0].id);
        assert_eq!(engine.rules()[0].enabled, restored.rules()[0].enabled);
    }

    #[test]
    fn format_alert_age_just_now() {
        let at = Instant::now();
        assert_eq!(format_alert_age(at), "just now");
    }

    #[test]
    fn apply_preset_replaces_rules() {
        let mut engine = AlertEngine::from_stored_rules(&[]);
        engine.apply_preset(AlertPreset::SECURITY_WATCH);
        let rules = engine.rules();
        assert!(!rules.iter().find(|r| r.id == 1).unwrap().enabled);
        assert!(rules.iter().find(|r| r.id == 3).unwrap().enabled);
        assert_eq!(rules.iter().find(|r| r.id == 5).unwrap().sustained_secs, 10);
    }

    #[test]
    fn recent_event_count_respects_window() {
        let mut engine = AlertEngine::new();
        let snap = NetworkSnapshot::default();
        let conn = sample_conn(8, "104.18.18.125", false);
        engine.evaluate(&snap, std::slice::from_ref(&conn));
        assert_eq!(engine.recent_event_count(Duration::from_secs(60)), 1);
    }
}

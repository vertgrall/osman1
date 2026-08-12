use freya::prelude::Color;

use crate::adapters::friendly_adapter_name;
use crate::detail::ConnectionDetail;
use crate::lfo::Waveform;
use crate::network::InterfaceStats;
use crate::theme::Palette;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrafficCharacter {
    SteadyStream,
    BatchSync,
    PulseApi,
    DuplexInteractive,
    ChaoticMultiplex,
    ListenIdle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtocolKind {
    Tcp,
    Udp,
    Icmp,
    Mixed,
}

#[derive(Clone, Copy, Debug)]
pub struct CharacterStyle {
    pub waveform: Waveform,
    pub dashed: bool,
    pub dual_lane: bool,
    pub idle_baseline: bool,
}

impl TrafficCharacter {
    pub fn all() -> [Self; 6] {
        [
            Self::SteadyStream,
            Self::BatchSync,
            Self::PulseApi,
            Self::DuplexInteractive,
            Self::ChaoticMultiplex,
            Self::ListenIdle,
        ]
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::SteadyStream => "Steady Stream",
            Self::BatchSync => "Batch Sync",
            Self::PulseApi => "Pulse / API",
            Self::DuplexInteractive => "Duplex Interactive",
            Self::ChaoticMultiplex => "Chaotic Multiplex",
            Self::ListenIdle => "Listen / Idle",
        }
    }

    pub fn detection_hint(self) -> &'static str {
        match self {
            Self::SteadyStream => "Low variance",
            Self::BatchSync => "High peak/average",
            Self::PulseApi => "Periodic bursts",
            Self::DuplexInteractive => "Balanced rx + tx",
            Self::ChaoticMultiplex => "High variance",
            Self::ListenIdle => "Mostly idle",
        }
    }

    pub fn style(self) -> CharacterStyle {
        match self {
            Self::SteadyStream => CharacterStyle {
                waveform: Waveform::Sine,
                dashed: false,
                dual_lane: false,
                idle_baseline: false,
            },
            Self::BatchSync => CharacterStyle {
                waveform: Waveform::Saw,
                dashed: false,
                dual_lane: false,
                idle_baseline: false,
            },
            Self::PulseApi => CharacterStyle {
                waveform: Waveform::Square,
                dashed: false,
                dual_lane: false,
                idle_baseline: false,
            },
            Self::DuplexInteractive => CharacterStyle {
                waveform: Waveform::Sine,
                dashed: false,
                dual_lane: true,
                idle_baseline: false,
            },
            Self::ChaoticMultiplex => CharacterStyle {
                waveform: Waveform::Sine,
                dashed: true,
                dual_lane: false,
                idle_baseline: false,
            },
            Self::ListenIdle => CharacterStyle {
                waveform: Waveform::Square,
                dashed: false,
                dual_lane: false,
                idle_baseline: true,
            },
        }
    }

    pub fn demo_bps(self) -> f64 {
        match self {
            Self::SteadyStream => 820_000.0,
            Self::BatchSync => 640_000.0,
            Self::PulseApi => 120_000.0,
            Self::DuplexInteractive => 540_000.0,
            Self::ChaoticMultiplex => 710_000.0,
            Self::ListenIdle => 4_000.0,
        }
    }

    pub fn primary_color(self, palette: Palette) -> Color {
        match self {
            Self::SteadyStream => palette.receive,
            Self::BatchSync => palette.send,
            Self::PulseApi => palette.total,
            Self::DuplexInteractive => palette.receive,
            Self::ChaoticMultiplex => palette.total,
            Self::ListenIdle => palette.send,
        }
    }

    pub fn secondary_color(self, palette: Palette) -> Color {
        match self {
            Self::DuplexInteractive => palette.send,
            _ => self.primary_color(palette),
        }
    }

    pub fn waveform_for_lane(self, lane: crate::theme::ProcessLane) -> Waveform {
        use crate::theme::ProcessLane;
        match self {
            Self::SteadyStream | Self::DuplexInteractive => Waveform::Sine,
            Self::BatchSync => match lane {
                ProcessLane::Red => Waveform::Saw,
                ProcessLane::Blue => Waveform::Triangle,
                ProcessLane::Green => Waveform::Square,
            },
            Self::PulseApi | Self::ListenIdle => Waveform::Square,
            Self::ChaoticMultiplex => Waveform::Sine,
        }
    }

    pub fn sample_for_lane(self, lane: crate::theme::ProcessLane, phase: f32) -> f32 {
        let offset = lane.lfo_phase_offset();
        let p = phase + offset;
        let style = self.style();
        if style.idle_baseline {
            return Waveform::sample_idle(p);
        }
        if style.dashed {
            return Waveform::sample_chaotic(p);
        }
        let shaped = self.waveform_for_lane(lane).sample(p);
        match self.waveform_for_lane(lane) {
            Waveform::Square => {
                if shaped >= 0.0 {
                    1.0
                } else {
                    -1.0
                }
            }
            Waveform::Triangle => shaped.signum() * shaped.abs().powf(0.85),
            _ => shaped,
        }
    }

    pub fn scope_id(self) -> u64 {
        match self {
            Self::SteadyStream => 10_001,
            Self::BatchSync => 10_002,
            Self::PulseApi => 10_003,
            Self::DuplexInteractive => 10_004,
            Self::ChaoticMultiplex => 10_005,
            Self::ListenIdle => 10_006,
        }
    }
}

impl ProtocolKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Tcp => "TCP",
            Self::Udp => "UDP",
            Self::Icmp => "ICMP",
            Self::Mixed => "Mixed",
        }
    }

    pub fn detail(self) -> &'static str {
        match self {
            Self::Tcp => "Reliable",
            Self::Udp => "Best Effort",
            Self::Icmp => "Control",
            Self::Mixed => "Mixed protocols",
        }
    }

    pub fn dashed_sparkline(self) -> bool {
        matches!(self, Self::Udp | Self::Icmp)
    }
}

pub fn classify_interface(
    stats: &InterfaceStats,
    connections: &[ConnectionDetail],
) -> (TrafficCharacter, ProtocolKind) {
    let character = detect_character(stats, connections);
    let protocol = detect_protocol(connections);
    (character, protocol)
}

fn detect_character(stats: &InterfaceStats, connections: &[ConnectionDetail]) -> TrafficCharacter {
    let history = &stats.combined_history;
    let listen_heavy = connections.iter().filter(|c| c.state == "Listen").count()
        > connections.len().max(1) / 2;

    if stats.combined_bps < 800.0 || listen_heavy {
        return TrafficCharacter::ListenIdle;
    }

    let (avg, peak) = history_stats(history);
    let peak_ratio = if avg > 1.0 { peak / avg } else { peak.max(1.0) };

    let rx = stats.rx_bps.max(1.0);
    let tx = stats.tx_bps.max(1.0);
    let duplex_balance = rx.min(tx) / rx.max(tx);

    if stats.rx_bps > 2_000.0 && stats.tx_bps > 2_000.0 && duplex_balance > 0.28 {
        return TrafficCharacter::DuplexInteractive;
    }

    if stats.consistency > 0.72 && peak_ratio < 2.8 {
        return TrafficCharacter::SteadyStream;
    }

    if peak_ratio > 5.0 && avg < peak * 0.25 {
        return TrafficCharacter::PulseApi;
    }

    if peak_ratio > 3.2 && stats.consistency < 0.62 {
        return TrafficCharacter::BatchSync;
    }

    if stats.consistency < 0.48 || peak_ratio > 4.5 {
        return TrafficCharacter::ChaoticMultiplex;
    }

    TrafficCharacter::SteadyStream
}

fn detect_protocol(connections: &[ConnectionDetail]) -> ProtocolKind {
    if connections.is_empty() {
        return ProtocolKind::Tcp;
    }

    let mut tcp = 0usize;
    let mut udp = 0usize;
    let mut icmp = 0usize;

    for conn in connections {
        let p = conn.protocol.to_ascii_lowercase();
        if p.contains("icmp") {
            icmp += 1;
        } else if p.contains("udp") {
            udp += 1;
        } else {
            tcp += 1;
        }
    }

    let total = tcp + udp + icmp;
    if total == 0 {
        return ProtocolKind::Tcp;
    }

    let dominant = tcp.max(udp).max(icmp);
    let kinds = [tcp, udp, icmp]
        .iter()
        .filter(|&&n| n > total / 4)
        .count();
    if kinds > 1 {
        return ProtocolKind::Mixed;
    }

    if dominant == icmp {
        ProtocolKind::Icmp
    } else if dominant == udp {
        ProtocolKind::Udp
    } else {
        ProtocolKind::Tcp
    }
}

fn history_stats(history: &[f64]) -> (f64, f64) {
    if history.is_empty() {
        return (0.0, 0.0);
    }
    let sum: f64 = history.iter().sum();
    let avg = sum / history.len() as f64;
    let peak = history.iter().copied().fold(0.0_f64, f64::max);
    (avg, peak)
}

pub fn behavior_note(character: TrafficCharacter, iface_name: &str) -> String {
    let kind = friendly_adapter_name(iface_name);
    match (character, kind) {
        (TrafficCharacter::ChaoticMultiplex, "Wi-Fi") => {
            "Mixed web + background sync".into()
        }
        (TrafficCharacter::SteadyStream, "Ethernet") => "Large sustained download".into(),
        (TrafficCharacter::DuplexInteractive, "VPN") => "Video call + SSH".into(),
        (TrafficCharacter::PulseApi, "Loopback") => "Local API polling".into(),
        (TrafficCharacter::BatchSync, _) => "Chunked upload / sync batches".into(),
        (TrafficCharacter::ListenIdle, _) => "Listening ports, rare traffic".into(),
        (TrafficCharacter::SteadyStream, _) => "Smooth continuous throughput".into(),
        (TrafficCharacter::ChaoticMultiplex, _) => "Many concurrent flows".into(),
        (TrafficCharacter::DuplexInteractive, _) => "Balanced two-way traffic".into(),
        (TrafficCharacter::PulseApi, _) => "Short bursts, mostly idle".into(),
    }
}

pub fn top_talker_for_interface(
    connections: &[ConnectionDetail],
    processes: &[crate::detail::ProcessTraffic],
) -> String {
    let mut best_name = String::new();
    let mut best_bytes = 0u64;

    for proc in processes {
        let total = proc.combined_bytes();
        if total > best_bytes {
            best_bytes = total;
            best_name = proc.name.clone();
        }
    }

    if best_name.is_empty() {
        let mut per_host: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
        for conn in connections {
            let host = conn.process_name.clone();
            *per_host.entry(host).or_default() += conn.combined_bytes();
        }
        if let Some((name, _)) = per_host.into_iter().max_by_key(|(_, b)| *b) {
            best_name = name;
        }
    }

    if best_name.is_empty() {
        "—".into()
    } else {
        best_name
    }
}

pub fn connections_for_interface<'a>(
    interface: &str,
    connections: &'a [ConnectionDetail],
) -> Vec<&'a ConnectionDetail> {
    connections
        .iter()
        .filter(|c| c.interface == interface)
        .collect()
}

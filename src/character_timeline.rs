use std::collections::HashMap;

use freya::components::CanvasContext;
use freya::engine::prelude::{Color4f, Paint, PaintStyle, Rect as SkRect, RRect};
use freya::prelude::Color;

use crate::detail::ConnectionDetail;
use crate::network::NetworkSnapshot;
use crate::theme::Palette;
use crate::time_window::TimeWindow;
use crate::traffic_character::{classify_interface, TrafficCharacter};

const MAX_SEGMENTS: usize = 64;

#[derive(Clone, Copy, Debug)]
pub struct CharacterSegment {
    pub character: TrafficCharacter,
    pub start_sample: u64,
    pub end_sample: Option<u64>,
}

#[derive(Clone, Default)]
pub struct CharacterTimeline {
    sample_index: u64,
    segments: HashMap<String, Vec<CharacterSegment>>,
}

impl CharacterTimeline {
    pub fn sample_index(&self) -> u64 {
        self.sample_index
    }

    pub fn observe_snapshot(&mut self, snapshot: &NetworkSnapshot, connections: &[ConnectionDetail]) {
        self.sample_index = self.sample_index.saturating_add(1);

        for iface in &snapshot.interfaces {
            let iface_conns: Vec<ConnectionDetail> = connections
                .iter()
                .filter(|c| c.interface == iface.name)
                .cloned()
                .collect();
            let (character, _) = classify_interface(iface, &iface_conns);
            self.observe_interface(&iface.name, character);
        }
    }

    fn observe_interface(&mut self, name: &str, character: TrafficCharacter) {
        let segs = self.segments.entry(name.to_string()).or_default();

        if let Some(last) = segs.last_mut() {
            if last.end_sample.is_none() && last.character == character {
                return;
            }
            if last.end_sample.is_none() {
                last.end_sample = Some(self.sample_index);
            }
        }

        segs.push(CharacterSegment {
            character,
            start_sample: self.sample_index,
            end_sample: None,
        });

        if segs.len() > MAX_SEGMENTS {
            segs.remove(0);
        }
    }

    pub fn segments_for(&self, iface: &str, window: TimeWindow) -> Vec<CharacterSegment> {
        let Some(segs) = self.segments.get(iface) else {
            return Vec::new();
        };

        let start = self.sample_index.saturating_sub(window.samples() as u64);
        segs.iter()
            .filter(|s| s.end_sample.unwrap_or(self.sample_index) >= start)
            .copied()
            .collect()
    }

    pub fn latest_transition_label(&self, iface: &str) -> Option<String> {
        let segs = self.segments.get(iface)?;
        if segs.len() < 2 {
            return None;
        }
        let prev = segs[segs.len() - 2];
        let curr = segs[segs.len() - 1];
        Some(format!(
            "{} → {}",
            prev.character.title(),
            curr.character.title()
        ))
    }
}

pub fn draw_character_timeline(
    ctx: &mut CanvasContext,
    segments: &[CharacterSegment],
    sample_index: u64,
    window: TimeWindow,
    palette: Palette,
) {
    let width = ctx.size.width.max(1.0);
    let height = ctx.size.height.max(1.0);

    let track = RRect::new_rect_xy(SkRect::from_xywh(0.0, 0.0, width, height), 4.0, 4.0);
    let mut track_paint = Paint::default();
    track_paint.set_style(PaintStyle::Fill);
    track_paint.set_color4f(color4f(palette.bar_track, 1.0), None);
    ctx.canvas.draw_rrect(track, &track_paint);

    if segments.is_empty() {
        return;
    }

    let span = window.samples() as u64;
    let window_start = sample_index.saturating_sub(span);

    for seg in segments {
        let end = seg.end_sample.unwrap_or(sample_index);
        if end < window_start {
            continue;
        }

        let start = seg.start_sample.max(window_start);
        let x0 = ((start - window_start) as f32 / span as f32) * width;
        let x1 = ((end - window_start) as f32 / span as f32) * width;
        let seg_w = (x1 - x0).max(2.0);

        let color = seg.character.primary_color(palette);
        let mut paint = Paint::default();
        paint.set_style(PaintStyle::Fill);
        paint.set_color4f(color4f(color, 0.85), None);
        ctx.canvas.draw_rect(
            SkRect::from_xywh(x0, height * 0.25, seg_w, height * 0.5),
            &paint,
        );
    }
}

fn color4f(color: Color, alpha_scale: f32) -> Color4f {
    Color4f::new(
        color.r() as f32 / 255.0,
        color.g() as f32 / 255.0,
        color.b() as f32 / 255.0,
        (color.a() as f32 / 255.0) * alpha_scale,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::InterfaceStats;

    fn stats(name: &str, bps: f64, consistency: f64) -> InterfaceStats {
        let hist = vec![bps; 16];
        InterfaceStats {
            name: name.into(),
            rx_bps: bps,
            tx_bps: 0.0,
            combined_bps: bps,
            total_rx: 0,
            total_tx: 0,
            consistency,
            heavy_consistent: false,
            rx_history: hist.clone(),
            tx_history: vec![0.0; 16],
            combined_history: hist,
        }
    }

    fn snapshot(iface: InterfaceStats) -> NetworkSnapshot {
        NetworkSnapshot {
            interfaces: vec![iface],
            ..NetworkSnapshot::default()
        }
    }

    #[test]
    fn timeline_records_character_transitions() {
        let mut timeline = CharacterTimeline::default();
        timeline.observe_snapshot(&snapshot(stats("en0", 100.0, 0.4)), &[]);
        timeline.observe_snapshot(&snapshot(stats("en0", 5_000_000.0, 0.9)), &[]);

        let label = timeline
            .latest_transition_label("en0")
            .expect("transition");
        assert!(
            label.contains("→"),
            "expected class transition arrow: {label}"
        );
        assert!(
            label.contains("Listen / Idle") || label.contains("Steady Stream"),
            "unexpected transition: {label}"
        );

        let segs = timeline.segments_for("en0", TimeWindow::Sec60);
        assert!(
            segs.len() >= 2,
            "expected at least two timeline segments, got {segs:?}"
        );
    }
}

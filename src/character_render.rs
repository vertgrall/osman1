//! Demo scopes and character-styled sparklines for Traffic Character view.

use std::collections::HashMap;
use std::f32::consts::TAU;

use freya::components::CanvasContext;
use freya::engine::prelude::{
    Color4f, Paint, PaintStyle, PathBuilder, Point, Rect as SkRect, RRect,
};
use freya::prelude::Color;

use crate::lfo::{bps_to_hz, Waveform};
use crate::particles::{
    draw_particle_field, is_chaotic_traffic, is_heavy_traffic, is_static_traffic, ParticleField,
};
use crate::clinical_render::{
    draw_clinical_bipolar_trace, draw_clinical_grid, draw_clinical_history_series,
    draw_clinical_plot_bg, draw_dial_gauge, ClinicalStyle,
};
use crate::theme::Palette;
use crate::traffic_character::{CharacterStyle, ProtocolKind, TrafficCharacter};

const BUFFER_LEN: usize = 128;
const TRACE_SAMPLE_RATE: f32 = 240.0;

#[derive(Clone, Copy)]
pub struct CharacterDrawProfile {
    pub amp_scale: f32,
    pub fill_alpha: f32,
    pub stroke_width: f32,
    pub glow: bool,
    pub scope_grid: bool,
    pub axis_gutter: f32,
    pub show_dial: bool,
}

impl CharacterDrawProfile {
    pub const LEGEND: Self = Self {
        amp_scale: 0.42,
        fill_alpha: 0.14,
        stroke_width: 1.25,
        glow: true,
        scope_grid: true,
        axis_gutter: 34.0,
        show_dial: true,
    };

    pub const ADAPTER_ROW: Self = Self {
        amp_scale: 0.46,
        fill_alpha: 0.12,
        stroke_width: 1.15,
        glow: true,
        scope_grid: true,
        axis_gutter: 28.0,
        show_dial: false,
    };
}

#[derive(Default)]
pub struct CharacterScopeBank {
    traces: HashMap<u64, CharacterTrace>,
}

impl CharacterScopeBank {
    pub fn draw_demo(
        &mut self,
        ctx: &mut CanvasContext,
        character: TrafficCharacter,
        time_secs: f64,
        palette: Palette,
    ) {
        self.draw_live(
            ctx,
            character.scope_id(),
            character,
            time_secs,
            palette,
            CharacterDrawProfile::LEGEND,
            None,
        );
    }

    pub fn draw_live(
        &mut self,
        ctx: &mut CanvasContext,
        trace_key: u64,
        character: TrafficCharacter,
        time_secs: f64,
        palette: Palette,
        profile: CharacterDrawProfile,
        rate_bps: Option<f64>,
    ) {
        let trace = self.traces.entry(trace_key).or_default();
        let bps = rate_bps.unwrap_or_else(|| character.demo_bps());
        trace.advance(character, time_secs, bps);
        trace.draw_with_profile(ctx, character, palette, profile);
    }
}

#[derive(Clone)]
struct CharacterTrace {
    samples: [f32; BUFFER_LEN],
    samples_b: [f32; BUFFER_LEN],
    phase: f32,
    phase_b: f32,
    last_time: Option<f64>,
}

impl Default for CharacterTrace {
    fn default() -> Self {
        Self {
            samples: [0.0; BUFFER_LEN],
            samples_b: [0.0; BUFFER_LEN],
            phase: 0.0,
            phase_b: 0.0,
            last_time: None,
        }
    }
}

impl CharacterTrace {
    fn advance(&mut self, character: TrafficCharacter, time_secs: f64, bps: f64) {
        let dt = match self.last_time {
            Some(prev) => (time_secs - prev).clamp(0.0, 0.05) as f32,
            None => 0.008,
        };
        self.last_time = Some(time_secs);
        if dt <= 0.0 {
            return;
        }

        let style = character.style();
        let hz = bps_to_hz(bps.max(800.0));
        let envelope = ((bps / 200_000.0).sqrt() as f32).clamp(0.22, 1.0);
        let steps = (dt * TRACE_SAMPLE_RATE).ceil().max(1.0) as usize;
        let step_dt = dt / steps as f32;

        for _ in 0..steps {
            self.phase += hz * step_dt * TAU;
            self.phase_b += hz * 1.35 * step_dt * TAU + 0.4;

            let primary = sample_for_style(style, self.phase, true);
            let secondary = if style.dual_lane {
                Waveform::Sine.sample(self.phase_b + TAU * 0.5)
            } else {
                0.0
            };

            self.samples.copy_within(1..BUFFER_LEN, 0);
            self.samples_b.copy_within(1..BUFFER_LEN, 0);
            self.samples[BUFFER_LEN - 1] = primary * envelope;
            self.samples_b[BUFFER_LEN - 1] = secondary * envelope * 0.85;
        }
    }

    fn draw_with_profile(
        &self,
        ctx: &mut CanvasContext,
        character: TrafficCharacter,
        palette: Palette,
        profile: CharacterDrawProfile,
    ) {
        let style = character.style();
        let width = ctx.size.width.max(1.0);
        let height = ctx.size.height.max(1.0);
        let plot_x = profile.axis_gutter;
        let plot_w = (width - plot_x).max(1.0);
        let mid = height * 0.5;
        let amp = height * profile.amp_scale;

        draw_track(ctx, plot_x, plot_w, width, height, palette, profile.scope_grid);

        if style.idle_baseline {
            let time = self.last_time.unwrap_or(0.0);
            draw_particle_field(
                ctx,
                ParticleField::idle(
                    character.scope_id(),
                    time,
                    Some(character.primary_color(palette)),
                ),
                palette,
            );
            draw_idle_baseline(ctx, plot_x, plot_w, mid, character.primary_color(palette));
            return;
        }

        if style.dual_lane {
            draw_clinical_bipolar_trace(
                ctx,
                plot_x,
                plot_w,
                &self.samples_b,
                mid,
                amp,
                character.secondary_color(palette),
                false,
                ClinicalStyle::SCOPE,
            );
        }

        if profile.fill_alpha > 0.0 {
            draw_trace_fill(
                ctx,
                plot_x,
                plot_w,
                &self.samples,
                mid,
                amp,
                character.primary_color(palette),
                profile.fill_alpha,
            );
        }

        if profile.glow {
            draw_clinical_bipolar_trace(
                ctx,
                plot_x,
                plot_w,
                &self.samples,
                mid,
                amp * 1.08,
                character.primary_color(palette),
                style.dashed,
                ClinicalStyle::SCOPE,
            );
        }

        draw_clinical_bipolar_trace(
            ctx,
            plot_x,
            plot_w,
            &self.samples,
            mid,
            amp,
            character.primary_color(palette),
            style.dashed,
            ClinicalStyle::SCOPE,
        );

        if profile.show_dial {
            let energy = self.samples[BUFFER_LEN - 1].abs().clamp(0.05, 1.0);
            draw_dial_gauge(
                ctx,
                width - 18.0,
                18.0,
                12.0,
                energy,
                character.primary_color(palette),
                palette,
            );
        }
    }
}

fn sample_for_style(style: CharacterStyle, phase: f32, _primary: bool) -> f32 {
    if style.idle_baseline {
        return Waveform::sample_idle(phase);
    }
    if style.dashed {
        return Waveform::sample_chaotic(phase);
    }
    style.waveform.sample(phase)
}

fn draw_track(
    ctx: &mut CanvasContext,
    plot_x: f32,
    plot_w: f32,
    width: f32,
    height: f32,
    palette: Palette,
    scope_grid: bool,
) {
    draw_clinical_plot_bg(ctx, 0.0, 0.0, width, height, palette);
    if scope_grid {
        let floor = height - 4.0;
        let plot_h = (floor - 4.0).max(8.0);
        draw_clinical_grid(ctx, plot_x, plot_w, floor, plot_h, palette, None);
    }
}

fn draw_trace_fill(
    ctx: &mut CanvasContext,
    plot_x: f32,
    plot_w: f32,
    samples: &[f32; BUFFER_LEN],
    mid: f32,
    amp: f32,
    color: Color,
    alpha: f32,
) {
    let mut path = PathBuilder::new();
    path.move_to(Point::new(plot_x, mid));
    for (i, sample) in samples.iter().enumerate() {
        let t = i as f32 / (BUFFER_LEN.saturating_sub(1).max(1) as f32);
        let x = plot_x + t * plot_w;
        let y = mid - *sample * amp;
        path.line_to(Point::new(x, y));
    }
    path.line_to(Point::new(plot_x + plot_w, mid));
    path.close();

    let mut paint = Paint::default();
    paint.set_style(PaintStyle::Fill);
    paint.set_anti_alias(true);
    paint.set_color4f(
        Color4f::new(
            color.r() as f32 / 255.0,
            color.g() as f32 / 255.0,
            color.b() as f32 / 255.0,
            alpha,
        ),
        None,
    );
    ctx.canvas.draw_path(&path.detach(), &paint);
}

fn draw_dashed_horizontal(canvas: &freya::engine::prelude::Canvas, width: f32, y: f32, paint: &Paint) {
    let mut x = 0.0;
    while x < width {
        let end = (x + 8.0).min(width);
        canvas.draw_line(Point::new(x, y), Point::new(end, y), paint);
        x += 11.0;
    }
}

pub fn draw_character_sparkline(
    ctx: &mut CanvasContext,
    history: &[f64],
    character: TrafficCharacter,
    protocol: ProtocolKind,
    palette: Palette,
    time_secs: f64,
    combined_bps: f64,
) {
    let width = ctx.size.width.max(1.0);
    let height = ctx.size.height.max(1.0);
    let top_pad = 3.0;
    let bottom_pad = 3.0;
    let floor = height - bottom_pad;
    let plot_h = (height - top_pad - bottom_pad).max(1.0);
    const AMP_GAIN: f32 = 1.55;
    let color = character.primary_color(palette);
    let dashed = protocol.dashed_sparkline() || character.style().dashed;

    draw_clinical_plot_bg(ctx, 0.0, 0.0, width, height, palette);

    if is_static_traffic(combined_bps, character) {
        let seed = history.len() as u64 ^ character.scope_id();
        draw_particle_field(
            ctx,
            ParticleField::idle(seed, time_secs, Some(color)),
            palette,
        );
        draw_idle_baseline(ctx, 0.0, width, floor, color);
        return;
    }

    if history.len() < 2 {
        draw_flat_baseline(ctx, width, floor, palette);
        return;
    }

    let max = history.iter().copied().fold(1.0_f64, f64::max) * 0.72;
    let scaled: Vec<f64> = history
        .iter()
        .map(|v| v * AMP_GAIN as f64)
        .collect();

    draw_clinical_grid(ctx, 0.0, width, floor, plot_h, palette, None);
    draw_clinical_history_series(
        ctx,
        &scaled,
        max,
        0.0,
        width,
        floor,
        plot_h,
        color,
        ClinicalStyle::SPARKLINE,
    );

    let live_fill = ((combined_bps / max.max(1.0)) as f32).clamp(0.0, 1.0);
    draw_dial_gauge(ctx, width - 14.0, 14.0, 10.0, live_fill, color, palette);

    let _ = dashed;

    if matches!(protocol, ProtocolKind::Icmp) {
        let steps = history.len();
        for (i, &v) in history.iter().enumerate().step_by(8) {
            let x = (i as f32 / (steps - 1) as f32) * width;
            let wave = ((v / max) as f32 * plot_h * AMP_GAIN).min(plot_h);
            let y = floor - wave;
            draw_diamond(ctx, Point::new(x, y), color);
        }
    }

    let seed = history.len() as u64 ^ character.scope_id();
    if is_heavy_traffic(combined_bps, false) {
        let intensity = (combined_bps / 400_000.0).clamp(0.35, 1.0) as f32;
        draw_particle_field(
            ctx,
            ParticleField::heavy(seed, time_secs, intensity),
            palette,
        );
    }
    if is_chaotic_traffic(character) {
        draw_particle_field(
            ctx,
            ParticleField::chaotic(seed.wrapping_add(17), time_secs),
            palette,
        );
    }
}

fn draw_idle_baseline(ctx: &mut CanvasContext, plot_x: f32, plot_w: f32, y: f32, color: Color) {
    let mut paint = Paint::default();
    paint.set_style(PaintStyle::Stroke);
    paint.set_stroke_width(1.0);
    paint.set_color4f(color4f(color, 0.35), None);
    ctx.canvas.draw_line(
        Point::new(plot_x, y),
        Point::new(plot_x + plot_w, y),
        &paint,
    );
}

fn draw_flat_baseline(ctx: &mut CanvasContext, width: f32, y: f32, palette: Palette) {
    let mut paint = Paint::default();
    paint.set_style(PaintStyle::Stroke);
    paint.set_stroke_width(1.0);
    paint.set_color4f(color4f(palette.muted, 0.35), None);
    ctx.canvas.draw_line(Point::new(0.0, y), Point::new(width, y), &paint);
}

fn draw_diamond(ctx: &mut CanvasContext, center: Point, color: Color) {
    let s = 2.5;
    let mut path = PathBuilder::new();
    path.move_to(Point::new(center.x, center.y - s));
    path.line_to(Point::new(center.x + s, center.y));
    path.line_to(Point::new(center.x, center.y + s));
    path.line_to(Point::new(center.x - s, center.y));
    path.close();
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_style(PaintStyle::Fill);
    paint.set_color4f(color4f(color, 0.9), None);
    ctx.canvas.draw_path(&path.detach(), &paint);
}

fn color4f(color: Color, alpha_scale: f32) -> Color4f {
    Color4f::new(
        color.r() as f32 / 255.0,
        color.g() as f32 / 255.0,
        color.b() as f32 / 255.0,
        (color.a() as f32 / 255.0) * alpha_scale,
    )
}

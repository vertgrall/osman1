//! Diffused square particles for idle / static traffic visuals.

use std::f32::consts::TAU;

use freya::components::CanvasContext;
use freya::engine::prelude::{Color4f, Paint, PaintStyle, Rect as SkRect};
use freya::prelude::Color;

use crate::theme::Palette;

#[derive(Clone, Copy, Debug)]
pub struct ParticleField {
    pub seed: u64,
    pub time: f64,
    /// 0..1 — density and brightness.
    pub intensity: f32,
    /// Drift speed multiplier (use ~0.12 for static / idle).
    pub speed: f32,
    /// Optional accent tint (e.g. green for Listen / Idle).
    pub tint: Option<Color>,
}

impl ParticleField {
    pub fn idle(seed: u64, time: f64, tint: Option<Color>) -> Self {
        Self {
            seed,
            time,
            intensity: 0.38,
            speed: 0.14,
            tint,
        }
    }

    pub fn heavy(seed: u64, time: f64, intensity: f32) -> Self {
        Self {
            seed,
            time,
            intensity: intensity.clamp(0.35, 1.0),
            speed: 0.55,
            tint: None,
        }
    }

    pub fn chaotic(seed: u64, time: f64) -> Self {
        Self {
            seed,
            time,
            intensity: 0.72,
            speed: 1.1,
            tint: None,
        }
    }
}

pub fn is_heavy_traffic(combined_bps: f64, heavy_consistent: bool) -> bool {
    heavy_consistent || combined_bps >= 40_000.0
}

pub fn is_chaotic_traffic(character: crate::traffic_character::TrafficCharacter) -> bool {
    character == crate::traffic_character::TrafficCharacter::ChaoticMultiplex
}

pub fn is_static_traffic(combined_bps: f64, character: crate::traffic_character::TrafficCharacter) -> bool {
    combined_bps < 800.0 || character == crate::traffic_character::TrafficCharacter::ListenIdle
}

pub fn draw_particle_field(ctx: &mut CanvasContext, field: ParticleField, palette: Palette) {
    let width = ctx.size.width.max(1.0);
    let height = ctx.size.height.max(1.0);
    let count = (38.0 * field.intensity.clamp(0.08, 1.0)).ceil() as usize;
    let dark = palette.bg.r() < 128;

    for i in 0..count {
        let h0 = hash_f32(field.seed, i);
        let h1 = hash_f32(field.seed, i + 97);
        let h2 = hash_f32(field.seed, i + 193);
        let h3 = hash_f32(field.seed, i + 311);

        let base_x = h0 * width;
        let base_y = h1 * height;
        let drift_x = (field.time as f32 * field.speed + h2 * TAU).sin()
            * (width * 0.06 + 4.0)
            * (0.5 + field.speed);
        let drift_y = (field.time as f32 * field.speed * 0.82 + h3 * TAU).cos()
            * (height * 0.08 + 3.0)
            * (0.5 + field.speed);

        let x = (base_x + drift_x).rem_euclid(width);
        let y = (base_y + drift_y).rem_euclid(height);
        let size = 1.2 + h2 * 2.2;
        let alpha = (0.06 + h3 * 0.18) * field.intensity;

        let (mut r, mut g, mut b) = if dark {
            (255u8, 255u8, 255u8)
        } else {
            (0u8, 0u8, 0u8)
        };

        if let Some(tint) = field.tint {
            let mix = 0.28;
            r = lerp_u8(r, tint.r(), mix);
            g = lerp_u8(g, tint.g(), mix);
            b = lerp_u8(b, tint.b(), mix);
        }

        draw_diffused_square(ctx, x, y, size, r, g, b, alpha);
    }
}

fn draw_diffused_square(
    ctx: &mut CanvasContext,
    x: f32,
    y: f32,
    size: f32,
    r: u8,
    g: u8,
    b: u8,
    alpha: f32,
) {
    for layer in 0..3 {
        let expand = layer as f32 * 0.7;
        let half = size * 0.5 + expand;
        let a = alpha * (1.0 - layer as f32 * 0.32);

        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        paint.set_style(PaintStyle::Fill);
        paint.set_color4f(
            Color4f::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a),
            None,
        );

        ctx.canvas.draw_rect(
            SkRect::from_xywh(x - half, y - half, half * 2.0, half * 2.0),
            &paint,
        );
    }
}

fn hash_f32(seed: u64, index: usize) -> f32 {
    let mut h = seed.wrapping_add((index as u64).wrapping_mul(9_781));
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51_afd7_ed55_8ccd);
    h ^= h >> 33;
    (h as f32 / u64::MAX as f32).clamp(0.0, 0.9999)
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t).round() as u8
}

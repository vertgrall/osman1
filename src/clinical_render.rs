//! Style 02 — Clinical Instrument: hairline ECG traces, labeled grid, dial gauges.

use freya::components::CanvasContext;
use freya::engine::prelude::{Color4f, Paint, PaintStyle, PathBuilder, Point, Rect as SkRect};
use freya::prelude::Color;

use crate::theme::{format_rate, Palette};

#[derive(Clone, Copy)]
pub struct ClinicalStyle {
    pub stroke_width: f32,
    pub dot_stride: usize,
}

impl ClinicalStyle {
    pub const SCOPE: Self = Self {
        stroke_width: 1.15,
        dot_stride: 6,
    };

    pub const SPARKLINE: Self = Self {
        stroke_width: 1.0,
        dot_stride: 8,
    };

    pub const HERO: Self = Self {
        stroke_width: 1.25,
        dot_stride: 5,
    };
}

pub fn draw_clinical_plot_bg(
    ctx: &mut CanvasContext,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    palette: Palette,
) {
    let mut paint = Paint::default();
    paint.set_style(PaintStyle::Fill);
    paint.set_color4f(color4f(palette.panel, 1.0), None);
    ctx.canvas.draw_rect(SkRect::from_xywh(x, y, w, h), &paint);
}

pub fn draw_clinical_grid(
    ctx: &mut CanvasContext,
    origin_x: f32,
    plot_w: f32,
    floor: f32,
    plot_h: f32,
    palette: Palette,
    _y_max_label: Option<&str>,
) {
    let (gr, gg, gb) = palette.chart_grid;
    let mut grid = Paint::default();
    grid.set_style(PaintStyle::Stroke);
    grid.set_stroke_width(1.0);
    grid.set_color4f(
        Color4f::new(gr as f32 / 255.0, gg as f32 / 255.0, gb as f32 / 255.0, 0.55),
        None,
    );

    for i in 0..=4 {
        let y = floor - (i as f32 / 4.0) * plot_h;
        ctx.canvas.draw_line(
            Point::new(origin_x, y),
            Point::new(origin_x + plot_w, y),
            &grid,
        );
    }
    for i in 0..=4 {
        let x = origin_x + (i as f32 / 4.0) * plot_w;
        ctx.canvas.draw_line(Point::new(x, floor), Point::new(x, floor - plot_h), &grid);
    }

    let mut tick = Paint::default();
    tick.set_style(PaintStyle::Stroke);
    tick.set_stroke_width(1.0);
    tick.set_color4f(color4f(palette.muted, 0.55), None);

    for i in 0..=4 {
        let y = floor - (i as f32 / 4.0) * plot_h;
        ctx.canvas.draw_line(
            Point::new(origin_x - 5.0, y),
            Point::new(origin_x, y),
            &tick,
        );
    }
    for i in 0..=4 {
        let x = origin_x + (i as f32 / 4.0) * plot_w;
        ctx.canvas.draw_line(
            Point::new(x, floor),
            Point::new(x, floor + 4.0),
            &tick,
        );
    }
}

pub fn draw_clinical_history_series(
    ctx: &mut CanvasContext,
    values: &[f64],
    max_y: f64,
    origin_x: f32,
    width: f32,
    floor: f32,
    plot_h: f32,
    color: Color,
    style: ClinicalStyle,
) {
    if values.len() < 2 || max_y <= 0.0 || width <= 0.0 {
        return;
    }

    let steps = values.len();
    let mut path = PathBuilder::new();

    for (i, &v) in values.iter().enumerate() {
        let x = origin_x + (i as f32 / (steps - 1) as f32) * width;
        let wave = ((v / max_y) as f32 * plot_h).clamp(0.0, plot_h);
        let y = floor - wave;
        if i == 0 {
            path.move_to(Point::new(x, y));
        } else {
            path.line_to(Point::new(x, y));
        }
    }

    let wave_path = path.detach();
    let mut stroke = Paint::default();
    stroke.set_anti_alias(true);
    stroke.set_style(PaintStyle::Stroke);
    stroke.set_stroke_width(style.stroke_width);
    stroke.set_color4f(color4f(color, 0.92), None);
    ctx.canvas.draw_path(&wave_path, &stroke);

    let mut dot = Paint::default();
    dot.set_anti_alias(true);
    dot.set_style(PaintStyle::Fill);
    dot.set_color4f(color4f(color, 0.95), None);

    for (i, &v) in values.iter().enumerate().step_by(style.dot_stride.max(1)) {
        let x = origin_x + (i as f32 / (steps - 1) as f32) * width;
        let wave = ((v / max_y) as f32 * plot_h).clamp(0.0, plot_h);
        let y = floor - wave;
        ctx.canvas.draw_circle(Point::new(x, y), 1.6, &dot);
    }
}

pub fn draw_clinical_bipolar_trace(
    ctx: &mut CanvasContext,
    plot_x: f32,
    plot_w: f32,
    samples: &[f32],
    mid: f32,
    amp: f32,
    color: Color,
    dashed: bool,
    style: ClinicalStyle,
) {
    if samples.is_empty() || plot_w <= 0.0 {
        return;
    }

    let steps = plot_w as usize;
    let mut path = PathBuilder::new();

    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let x = plot_x + t * plot_w;
        let y = mid - sample_slice(samples, t) * amp;
        if i == 0 {
            path.move_to(Point::new(x, y));
        } else {
            path.line_to(Point::new(x, y));
        }
    }

    let wave = path.detach();
    let mut stroke = Paint::default();
    stroke.set_anti_alias(true);
    stroke.set_style(PaintStyle::Stroke);
    stroke.set_stroke_width(style.stroke_width);
    stroke.set_color4f(color4f(color, 0.9), None);

    if dashed {
        let mut prev: Option<Point> = None;
        for i in 0..=steps {
            if i % 2 == 1 {
                continue;
            }
            let t = i as f32 / steps as f32;
            let pt = Point::new(
                plot_x + t * plot_w,
                mid - sample_slice(samples, t) * amp,
            );
            if let Some(from) = prev {
                ctx.canvas.draw_line(from, pt, &stroke);
            }
            prev = Some(pt);
        }
    } else {
        ctx.canvas.draw_path(&wave, &stroke);
    }

    let mut dot = Paint::default();
    dot.set_anti_alias(true);
    dot.set_style(PaintStyle::Fill);
    dot.set_color4f(color4f(color, 0.95), None);

    for i in (0..=steps).step_by(style.dot_stride.max(1)) {
        let t = i as f32 / steps as f32;
        let x = plot_x + t * plot_w;
        let y = mid - sample_slice(samples, t) * amp;
        ctx.canvas.draw_circle(Point::new(x, y), 1.5, &dot);
    }
}

/// Circular dial gauge — `fill` in 0…1.
pub fn draw_dial_gauge(
    ctx: &mut CanvasContext,
    cx: f32,
    cy: f32,
    radius: f32,
    fill: f32,
    color: Color,
    palette: Palette,
) {
    let fill = fill.clamp(0.0, 1.0);
    let start = std::f32::consts::PI * 0.75;
    let sweep = std::f32::consts::PI * 1.5;

    let mut track = Paint::default();
    track.set_anti_alias(true);
    track.set_style(PaintStyle::Stroke);
    track.set_stroke_width(2.5);
    track.set_color4f(color4f(palette.muted, 0.22), None);

    let mut track_path = PathBuilder::new();
    arc_points(&mut track_path, cx, cy, radius, start, start + sweep, 24);
    ctx.canvas.draw_path(&track_path.detach(), &track);

    if fill > 0.01 {
        let mut value = Paint::default();
        value.set_anti_alias(true);
        value.set_style(PaintStyle::Stroke);
        value.set_stroke_width(2.5);
        value.set_color4f(color4f(color, 0.92), None);

        let mut value_path = PathBuilder::new();
        arc_points(
            &mut value_path,
            cx,
            cy,
            radius,
            start,
            start + sweep * fill,
            (24.0 * fill).max(2.0) as usize,
        );
        ctx.canvas.draw_path(&value_path.detach(), &value);
    }

    let mut hub = Paint::default();
    hub.set_anti_alias(true);
    hub.set_style(PaintStyle::Fill);
    hub.set_color4f(color4f(palette.panel, 1.0), None);
    ctx.canvas.draw_circle(Point::new(cx, cy), 2.0, &hub);
}

pub fn y_max_label(max_y: f64) -> String {
    format_rate(max_y)
}

fn arc_points(path: &mut PathBuilder, cx: f32, cy: f32, r: f32, start: f32, end: f32, segments: usize) {
    let segments = segments.max(2);
    for i in 0..=segments {
        let t = start + (end - start) * i as f32 / segments as f32;
        let pt = Point::new(cx + t.cos() * r, cy + t.sin() * r);
        if i == 0 {
            path.move_to(pt);
        } else {
            path.line_to(pt);
        }
    }
}

fn sample_slice(values: &[f32], t: f32) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    if values.len() == 1 {
        return values[0];
    }
    let idx = t * (values.len() - 1) as f32;
    let i0 = idx.floor() as usize;
    let i1 = (i0 + 1).min(values.len() - 1);
    let frac = idx - i0 as f32;
    values[i0] * (1.0 - frac) + values[i1] * frac
}

fn color4f(color: Color, alpha_scale: f32) -> Color4f {
    Color4f::new(
        color.r() as f32 / 255.0,
        color.g() as f32 / 255.0,
        color.b() as f32 / 255.0,
        (color.a() as f32 / 255.0) * alpha_scale,
    )
}

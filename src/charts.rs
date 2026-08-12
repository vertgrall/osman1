//! Skia history charts — sparklines and area charts for Direction 3 layout.

use freya::components::CanvasContext;
use freya::engine::prelude::{
    ClipOp, Color4f, Font, Paint, PaintStyle, PathBuilder, Point, Rect as SkRect, RRect,
};
use freya::prelude::Color;
use skia_safe::utils::text_utils::Align as TextAlign;

use crate::theme::{format_rate, Palette};
use crate::time_window::TimeWindow;

pub const MIN_CHART_SCALE: f64 = 512.0;
const SCALE_BUMP_RATIO: f64 = 1.04;
/// Trailing-edge quiet threshold vs locked scale.
const TRAILING_QUIET_RATIO: f64 = 0.38;

/// Holds Y-axis until peaks scroll off, then tracks the trailing edge down.
#[derive(Clone, Debug, Default)]
pub struct StickyChartScale {
    locked: f64,
    window: Option<TimeWindow>,
}

#[derive(Clone, Debug, Default)]
pub struct ChartScaleBank {
    hero: StickyChartScale,
    adapters: std::collections::HashMap<u64, StickyChartScale>,
}

impl ChartScaleBank {
    pub fn hero_y(
        &mut self,
        window: TimeWindow,
        rx: &[f64],
        tx: &[f64],
        combined: &[f64],
    ) -> f64 {
        self.hero.resolve(window, rx, tx, combined)
    }

    pub fn adapter_y(
        &mut self,
        key: u64,
        window: TimeWindow,
        rx: &[f64],
        tx: &[f64],
        combined: &[f64],
    ) -> f64 {
        self.adapters
            .entry(key)
            .or_default()
            .resolve(window, rx, tx, combined)
    }
}

impl StickyChartScale {
    pub fn resolve(
        &mut self,
        window: TimeWindow,
        rx: &[f64],
        tx: &[f64],
        combined: &[f64],
    ) -> f64 {
        if self.window != Some(window) {
            self.window = Some(window);
            self.locked = 0.0;
        }

        let trailing_max = trailing_peak(combined, 8);
        let live = combined.last().copied().unwrap_or(0.0);
        let target = natural_scale(trailing_max.max(live), live);

        if self.locked <= 0.0 {
            self.locked = target;
            return self.locked;
        }

        // Only bump from recent traffic — not peaks scrolling off on the left.
        if trailing_max > self.locked * SCALE_BUMP_RATIO {
            self.locked = natural_scale(trailing_max.max(live), live);
            return self.locked;
        }

        let quiet_trailing = trailing_max < self.locked * TRAILING_QUIET_RATIO;
        let quiet_live = live < self.locked * 0.22;
        if quiet_trailing || quiet_live {
            let gap = self.locked / target.max(1.0);
            self.locked = if gap > 8.0 {
                target * 1.12
            } else if gap > 4.0 {
                target + (self.locked - target) * 0.28
            } else if gap > 2.0 {
                self.locked * 0.45 + target * 0.55
            } else {
                self.locked * 0.65 + target * 0.35
            };
            if (self.locked - target).abs() < target * 0.06 {
                self.locked = target;
            }
        }

        self.locked.max(MIN_CHART_SCALE)
    }
}

fn trailing_peak(values: &[f64], tail_samples: usize) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let tail = tail_samples.max(3).min(values.len());
    values
        .iter()
        .rev()
        .take(tail)
        .copied()
        .fold(0.0_f64, f64::max)
}

fn series_peak(rx: &[f64], tx: &[f64], combined: &[f64]) -> f64 {
    rx.iter()
        .chain(tx.iter())
        .chain(combined.iter())
        .copied()
        .fold(0.0_f64, f64::max)
}

fn natural_scale(relevant_max: f64, live: f64) -> f64 {
    let tail = relevant_max.max(live * 1.1);
    if tail <= 0.0 {
        return MIN_CHART_SCALE;
    }
    (tail * 1.18).max(live * 1.8).max(MIN_CHART_SCALE)
}

const CHART_LEFT: f32 = 46.0;
const CHART_BOTTOM: f32 = 20.0;
const SPARKLINE_TOP: f32 = 4.0;
const AXIS_FONT_SIZE: f32 = 9.0;

fn chart_axis_font(size: f32) -> Font {
    let mut font = Font::default();
    font.set_size(size);
    font
}

fn axis_label_paint(palette: Palette) -> Paint {
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    let (r, g, b) = palette.chart_label;
    paint.set_color4f(
        Color4f::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 0.92),
        None,
    );
    paint
}

fn clip_plot(ctx: &mut CanvasContext, width: f32, height: f32) {
    ctx.canvas.clip_rect(
        SkRect::from_xywh(0.0, 0.0, width, height),
        ClipOp::Intersect,
        true,
    );
}

/// Downsample long histories so mini charts stay readable and bounded.
fn resample_for_display(values: &[f64], max_points: usize) -> Vec<f64> {
    if values.len() <= max_points {
        return values.to_vec();
    }
    let bucket = values.len() as f64 / max_points as f64;
    (0..max_points)
        .map(|i| {
            let start = (i as f64 * bucket).floor() as usize;
            let end = (((i + 1) as f64 * bucket).ceil() as usize).min(values.len());
            values[start..end].iter().copied().fold(0.0_f64, f64::max)
        })
        .collect()
}

pub fn draw_sparkline(ctx: &mut CanvasContext, values: &[f64], line: Color, palette: Palette) {
    let width = ctx.size.width.max(1.0);
    let height = ctx.size.height.max(1.0);
    let floor = height * 0.82;

    let track = RRect::new_rect_xy(SkRect::from_xywh(0.0, 0.0, width, height), 4.0, 4.0);
    let mut track_paint = Paint::default();
    track_paint.set_style(PaintStyle::Fill);
    track_paint.set_color4f(color4f(palette.bar_track, 1.0), None);
    ctx.canvas.draw_rrect(track, &track_paint);

    if values.len() < 2 {
        draw_sparkline_baseline(ctx, 0.0, width, floor, palette);
        return;
    }

    let max = values.iter().copied().fold(1.0_f64, f64::max) * 1.1;
    let steps = values.len();
    let mut line_path = PathBuilder::new();
    let mut fill_path = PathBuilder::new();
    fill_path.move_to(Point::new(0.0, floor));

    for (i, &v) in values.iter().enumerate() {
        let x = (i as f32 / (steps - 1) as f32) * width;
        let y = floor - ((v / max) as f32 * (floor - 2.0));
        if i == 0 {
            line_path.move_to(Point::new(x, y));
        } else {
            line_path.line_to(Point::new(x, y));
        }
        fill_path.line_to(Point::new(x, y));
    }

    fill_path.line_to(Point::new(width, floor));
    fill_path.close();

    let fill = fill_path.detach();
    let mut fill_paint = Paint::default();
    fill_paint.set_anti_alias(true);
    fill_paint.set_style(PaintStyle::Fill);
    fill_paint.set_color4f(color4f(line, 0.22), None);
    ctx.canvas.draw_path(&fill, &fill_paint);

    let stroke = line_path.detach();
    let mut stroke_paint = Paint::default();
    stroke_paint.set_anti_alias(true);
    stroke_paint.set_style(PaintStyle::Stroke);
    stroke_paint.set_stroke_width(1.4);
    stroke_paint.set_color4f(color4f(line, 0.95), None);
    ctx.canvas.draw_path(&stroke, &stroke_paint);
}

/// Mini layered sparkline — receive (blue), send (green), total (purple).
pub fn draw_activity_sparkline(
    ctx: &mut CanvasContext,
    rx: &[f64],
    tx: &[f64],
    combined: &[f64],
    palette: Palette,
    max_y: f64,
) {
    let width = ctx.size.width.max(1.0);
    let height = ctx.size.height.max(1.0);
    let origin_x = 0.0;
    let plot_w = (width - 2.0).max(1.0);
    let floor = height - 3.0;
    let plot_h = (floor - SPARKLINE_TOP).max(4.0);
    let scale = if max_y > 0.0 { max_y } else { MIN_CHART_SCALE };

    ctx.canvas.save();
    clip_plot(ctx, width, height);

    let track = RRect::new_rect_xy(SkRect::from_xywh(0.0, 0.0, width, height), 4.0, 4.0);
    let mut track_paint = Paint::default();
    track_paint.set_style(PaintStyle::Fill);
    track_paint.set_color4f(color4f(palette.bar_track, 1.0), None);
    ctx.canvas.draw_rrect(track, &track_paint);

    let rx = resample_for_display(rx, 96);
    let tx = resample_for_display(tx, 96);
    let combined = resample_for_display(combined, 96);

    let len = rx.len().max(tx.len()).max(combined.len());
    if len < 2 {
        draw_sparkline_baseline(ctx, origin_x, plot_w, floor, palette);
        ctx.canvas.restore();
        return;
    }

    let draw_max = scale.max(1.0);

    draw_area_series_offset(
        ctx,
        &rx,
        draw_max,
        origin_x,
        plot_w,
        floor,
        plot_h,
        palette.receive,
        0.30,
    );
    draw_area_series_offset(
        ctx,
        &tx,
        draw_max,
        origin_x,
        plot_w,
        floor,
        plot_h,
        palette.send,
        0.24,
    );
    draw_line_series_offset(
        ctx,
        &combined,
        draw_max,
        origin_x,
        plot_w,
        floor,
        plot_h,
        palette.total,
        1.4,
    );
    ctx.canvas.restore();
}

fn pad_series_for_draw(
    rx: Vec<f64>,
    tx: Vec<f64>,
    combined: Vec<f64>,
) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let len = rx.len().max(tx.len()).max(combined.len());
    if len >= 2 {
        return (rx, tx, combined);
    }
    let pad = |values: Vec<f64>| {
        if values.is_empty() {
            vec![0.0, 0.0]
        } else {
            vec![values[0], values[0]]
        }
    };
    (pad(rx), pad(tx), pad(combined))
}

pub fn draw_network_activity(
    ctx: &mut CanvasContext,
    rx: &[f64],
    tx: &[f64],
    combined: &[f64],
    palette: Palette,
    window: TimeWindow,
    max_y: f64,
) {
    let width = ctx.size.width.max(1.0);
    let height = ctx.size.height.max(1.0);
    let plot_w = (width - CHART_LEFT - 4.0).max(1.0);
    let plot_h = (height - CHART_BOTTOM - 4.0).max(1.0);
    let floor = CHART_BOTTOM + plot_h;
    let origin_x = CHART_LEFT;

    ctx.canvas.save();
    clip_plot(ctx, width, height);

    fill_bg(ctx, width, height, palette);

    let rx = resample_for_display(rx, 180);
    let tx = resample_for_display(tx, 180);
    let combined = resample_for_display(combined, 180);

    let len = rx.len().max(tx.len()).max(combined.len());
    let label_scale = if max_y > 0.0 { max_y.max(1.0) } else { MIN_CHART_SCALE };
    if len == 0 {
        draw_idle_grid(ctx, origin_x, plot_w, floor, plot_h, palette);
        draw_axis_labels(
            ctx,
            origin_x,
            plot_w,
            floor,
            plot_h,
            label_scale,
            palette,
            window,
        );
        ctx.canvas.restore();
        return;
    }

    let (rx, tx, combined) = pad_series_for_draw(rx, tx, combined);
    let max_y = max_y.max(1.0);

    draw_grid_lines(ctx, origin_x, plot_w, floor, plot_h, palette);
    draw_area_series_offset(
        ctx,
        &rx,
        max_y,
        origin_x,
        plot_w,
        floor,
        plot_h,
        palette.receive,
        0.38,
    );
    draw_area_series_offset(
        ctx,
        &tx,
        max_y,
        origin_x,
        plot_w,
        floor,
        plot_h,
        palette.send,
        0.32,
    );
    draw_line_series_offset(
        ctx,
        &combined,
        max_y,
        origin_x,
        plot_w,
        floor,
        plot_h,
        palette.total,
        2.0,
    );
    draw_axis_labels(
        ctx,
        origin_x,
        plot_w,
        floor,
        plot_h,
        max_y,
        palette,
        window,
    );
    ctx.canvas.restore();
}

pub fn chart_y_labels(max_y: f64) -> [String; 3] {
    if max_y <= 0.0 {
        return ["0 B/s".into(), "—".into(), "—".into()];
    }
    [
        "0 B/s".into(),
        format_rate(max_y * 0.5),
        format_rate(max_y),
    ]
}

pub fn display_chart_max(locked: f64, rx: &[f64], tx: &[f64], combined: &[f64]) -> f64 {
    let peak = series_peak(rx, tx, combined);
    if peak <= 0.0 {
        return locked.max(MIN_CHART_SCALE);
    }
    let auto = natural_scale(peak, peak);
    if locked > auto * 1.5 {
        auto
    } else {
        locked.max(auto)
    }
}

pub fn sparkline_scale(rx: &[f64], tx: &[f64], combined: &[f64]) -> f64 {
    let peak = series_peak(rx, tx, combined);
    if peak <= 0.0 {
        return MIN_CHART_SCALE;
    }
    (peak * 1.25).max(64.0)
}

pub fn chart_peak_max(rx: &[f64], tx: &[f64], combined: &[f64]) -> f64 {
    natural_scale(series_peak(rx, tx, combined), combined.last().copied().unwrap_or(0.0))
}

/// Instant Y scale (no hold). Prefer [`StickyChartScale`] for live charts.
pub fn chart_scale_max(rx: &[f64], tx: &[f64], combined: &[f64]) -> f64 {
    chart_peak_max(rx, tx, combined)
}

fn draw_axis_labels(
    ctx: &mut CanvasContext,
    origin_x: f32,
    plot_w: f32,
    floor: f32,
    plot_h: f32,
    max_y: f64,
    palette: Palette,
    window: TimeWindow,
) {
    draw_axis_ticks(ctx, origin_x, plot_w, floor, plot_h, palette);
    draw_y_axis_labels(ctx, origin_x, floor, plot_h, max_y, palette);
    draw_x_axis_labels(ctx, origin_x, plot_w, floor, palette, window);
}

fn draw_y_axis_labels(
    ctx: &mut CanvasContext,
    origin_x: f32,
    floor: f32,
    plot_h: f32,
    max_y: f64,
    palette: Palette,
) {
    let labels = chart_y_labels(max_y);
    let font = chart_axis_font(AXIS_FONT_SIZE);
    let paint = axis_label_paint(palette);

    for (i, label) in labels.iter().enumerate() {
        let y = floor - (i as f32 / 2.0) * plot_h;
        ctx.canvas.draw_str_align(
            label.as_str(),
            Point::new(origin_x - 6.0, y + 3.0),
            &font,
            &paint,
            TextAlign::Right,
        );
    }
}

fn draw_x_axis_labels(
    ctx: &mut CanvasContext,
    origin_x: f32,
    plot_w: f32,
    floor: f32,
    palette: Palette,
    window: TimeWindow,
) {
    let labels = window.x_labels();
    let font = chart_axis_font(AXIS_FONT_SIZE);
    let paint = axis_label_paint(palette);
    let aligns = [TextAlign::Left, TextAlign::Center, TextAlign::Right];

    for (i, label) in labels.iter().enumerate() {
        let x = origin_x + (i as f32 / 2.0) * plot_w;
        ctx.canvas.draw_str_align(
            label.as_str(),
            Point::new(x, floor + 14.0),
            &font,
            &paint,
            aligns[i],
        );
    }
}

fn draw_axis_ticks(
    ctx: &mut CanvasContext,
    origin_x: f32,
    plot_w: f32,
    floor: f32,
    plot_h: f32,
    palette: Palette,
) {
    let mut tick = Paint::default();
    tick.set_style(PaintStyle::Stroke);
    tick.set_stroke_width(1.0);
    tick.set_color4f(color4f(palette.muted, 0.45), None);

    for i in 0..3 {
        let y = floor - (i as f32 / 2.0) * plot_h;
        ctx.canvas
            .draw_line(Point::new(origin_x - 4.0, y), Point::new(origin_x, y), &tick);
    }

    for frac in [0.0, 0.5, 1.0] {
        let x = origin_x + frac * plot_w;
        ctx.canvas
            .draw_line(Point::new(x, floor), Point::new(x, floor + 4.0), &tick);
    }
}

fn draw_area_series_offset(
    ctx: &mut CanvasContext,
    values: &[f64],
    max_y: f64,
    origin_x: f32,
    width: f32,
    floor: f32,
    plot_h: f32,
    color: Color,
    fill_alpha: f32,
) {
    if values.len() < 2 {
        return;
    }

    let steps = values.len();
    let mut fill = PathBuilder::new();
    fill.move_to(Point::new(origin_x, floor));

    for (i, &v) in values.iter().enumerate() {
        let x = origin_x + (i as f32 / (steps - 1) as f32) * width;
        let wave = ((v / max_y) as f32 * plot_h).clamp(0.0, plot_h);
        let y = floor - wave;
        fill.line_to(Point::new(x, y));
    }

    fill.line_to(Point::new(origin_x + width, floor));
    fill.close();

    let path = fill.detach();
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_style(PaintStyle::Fill);
    paint.set_color4f(color4f(color, fill_alpha), None);
    ctx.canvas.draw_path(&path, &paint);

    let mut line = PathBuilder::new();
    for (i, &v) in values.iter().enumerate() {
        let x = origin_x + (i as f32 / (steps - 1) as f32) * width;
        let wave = ((v / max_y) as f32 * plot_h).clamp(0.0, plot_h);
        let y = floor - wave;
        if i == 0 {
            line.move_to(Point::new(x, y));
        } else {
            line.line_to(Point::new(x, y));
        }
    }

    let stroke = line.detach();
    let mut stroke_paint = Paint::default();
    stroke_paint.set_anti_alias(true);
    stroke_paint.set_style(PaintStyle::Stroke);
    stroke_paint.set_stroke_width(1.2);
    stroke_paint.set_color4f(color4f(color, 0.85), None);
    ctx.canvas.draw_path(&stroke, &stroke_paint);
}

fn draw_line_series_offset(
    ctx: &mut CanvasContext,
    values: &[f64],
    max_y: f64,
    origin_x: f32,
    width: f32,
    floor: f32,
    plot_h: f32,
    color: Color,
    stroke_w: f32,
) {
    if values.len() < 2 {
        return;
    }

    let steps = values.len();
    let mut line = PathBuilder::new();
    for (i, &v) in values.iter().enumerate() {
        let x = origin_x + (i as f32 / (steps - 1) as f32) * width;
        let wave = ((v / max_y) as f32 * plot_h).clamp(0.0, plot_h);
        let y = floor - wave;
        if i == 0 {
            line.move_to(Point::new(x, y));
        } else {
            line.line_to(Point::new(x, y));
        }
    }

    let path = line.detach();
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_style(PaintStyle::Stroke);
    paint.set_stroke_width(stroke_w);
    paint.set_color4f(color4f(color, 0.95), None);
    ctx.canvas.draw_path(&path, &paint);
}

fn draw_area_series(
    ctx: &mut CanvasContext,
    values: &[f64],
    max_y: f64,
    width: f32,
    floor: f32,
    plot_h: f32,
    color: Color,
    fill_alpha: f32,
) {
    if values.len() < 2 {
        return;
    }

    let steps = values.len();
    let mut fill = PathBuilder::new();
    fill.move_to(Point::new(0.0, floor));

    for (i, &v) in values.iter().enumerate() {
        let x = (i as f32 / (steps - 1) as f32) * width;
        let wave = ((v / max_y) as f32 * plot_h).clamp(0.0, plot_h);
        let y = floor - wave;
        fill.line_to(Point::new(x, y));
    }

    fill.line_to(Point::new(width, floor));
    fill.close();

    let path = fill.detach();
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_style(PaintStyle::Fill);
    paint.set_color4f(color4f(color, fill_alpha), None);
    ctx.canvas.draw_path(&path, &paint);

    let mut line = PathBuilder::new();
    for (i, &v) in values.iter().enumerate() {
        let x = (i as f32 / (steps - 1) as f32) * width;
        let wave = ((v / max_y) as f32 * plot_h).clamp(0.0, plot_h);
        let y = floor - wave;
        if i == 0 {
            line.move_to(Point::new(x, y));
        } else {
            line.line_to(Point::new(x, y));
        }
    }

    let stroke = line.detach();
    let mut stroke_paint = Paint::default();
    stroke_paint.set_anti_alias(true);
    stroke_paint.set_style(PaintStyle::Stroke);
    stroke_paint.set_stroke_width(1.2);
    stroke_paint.set_color4f(color4f(color, 0.85), None);
    ctx.canvas.draw_path(&stroke, &stroke_paint);
}

fn draw_line_series(
    ctx: &mut CanvasContext,
    values: &[f64],
    max_y: f64,
    width: f32,
    floor: f32,
    plot_h: f32,
    color: Color,
    stroke_w: f32,
) {
    if values.len() < 2 {
        return;
    }

    let steps = values.len();
    let mut line = PathBuilder::new();
    for (i, &v) in values.iter().enumerate() {
        let x = (i as f32 / (steps - 1) as f32) * width;
        let wave = ((v / max_y) as f32 * plot_h).clamp(0.0, plot_h);
        let y = floor - wave;
        if i == 0 {
            line.move_to(Point::new(x, y));
        } else {
            line.line_to(Point::new(x, y));
        }
    }

    let path = line.detach();
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_style(PaintStyle::Stroke);
    paint.set_stroke_width(stroke_w);
    paint.set_color4f(color4f(color, 0.95), None);
    ctx.canvas.draw_path(&path, &paint);
}

fn fill_bg(ctx: &mut CanvasContext, width: f32, height: f32, palette: Palette) {
    let (fr, fg, fb) = palette.chart_fill;
    let mut paint = Paint::default();
    paint.set_style(PaintStyle::Fill);
    paint.set_color4f(
        Color4f::new(fr as f32 / 255.0, fg as f32 / 255.0, fb as f32 / 255.0, 1.0),
        None,
    );
    ctx.canvas.draw_rect(SkRect::from_xywh(0.0, 0.0, width, height), &paint);
}

fn draw_idle_grid(
    ctx: &mut CanvasContext,
    origin_x: f32,
    plot_w: f32,
    floor: f32,
    plot_h: f32,
    palette: Palette,
) {
    draw_grid_lines(ctx, origin_x, plot_w, floor, plot_h, palette);
    draw_baseline(ctx, origin_x, plot_w, floor, palette);
}

fn draw_grid_lines(
    ctx: &mut CanvasContext,
    origin_x: f32,
    plot_w: f32,
    floor: f32,
    plot_h: f32,
    palette: Palette,
) {
    let (gr, gg, gb) = palette.chart_grid;
    let mut grid = Paint::default();
    grid.set_style(PaintStyle::Stroke);
    grid.set_stroke_width(1.0);
    grid.set_color4f(
        Color4f::new(gr as f32 / 255.0, gg as f32 / 255.0, gb as f32 / 255.0, 0.35),
        None,
    );

    for i in 1..4 {
        let y = floor - plot_h * i as f32 / 4.0;
        ctx.canvas.draw_line(
            Point::new(origin_x, y),
            Point::new(origin_x + plot_w, y),
            &grid,
        );
    }
}

fn draw_sparkline_baseline(
    ctx: &mut CanvasContext,
    origin_x: f32,
    plot_w: f32,
    y: f32,
    palette: Palette,
) {
    let mut paint = Paint::default();
    paint.set_style(PaintStyle::Stroke);
    paint.set_stroke_width(1.0);
    paint.set_color4f(color4f(palette.muted, 0.35), None);
    ctx.canvas.draw_line(
        Point::new(origin_x, y),
        Point::new(origin_x + plot_w, y),
        &paint,
    );
}

fn draw_baseline(ctx: &mut CanvasContext, origin_x: f32, width: f32, y: f32, palette: Palette) {
    let mut paint = Paint::default();
    paint.set_style(PaintStyle::Stroke);
    paint.set_stroke_width(1.0);
    paint.set_color4f(color4f(palette.muted, 0.35), None);
    ctx.canvas.draw_line(
        Point::new(origin_x, y),
        Point::new(origin_x + width, y),
        &paint,
    );
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

    #[test]
    fn display_chart_max_tracks_low_traffic() {
        let rx = vec![1000.0, 4800.0];
        let tx = vec![500.0, 2400.0];
        let combined = vec![1500.0, 7200.0];
        let locked = 50_000.0;
        let display = display_chart_max(locked, &rx, &tx, &combined);
        assert!(
            display < locked,
            "display scale should drop for low traffic: {display} vs locked {locked}"
        );
        assert!(display >= 7200.0 * 1.1);
    }

    #[test]
    fn chart_y_labels_shows_three_rate_ticks() {
        let labels = chart_y_labels(7200.0);
        assert_eq!(labels[0], "0 B/s");
        assert!(labels[1].contains("B/s"));
        assert!(labels[2].contains("B/s"));
        assert_ne!(labels[1], labels[2]);
    }

    #[test]
    fn sparkline_scale_visible_for_bytes_per_second_traffic() {
        let rx = vec![0.0, 60.0];
        let tx = vec![0.0, 0.0];
        let combined = vec![0.0, 60.0];
        let scale = sparkline_scale(&rx, &tx, &combined);
        assert!(scale < MIN_CHART_SCALE);
        assert!(scale >= 60.0);
    }

    #[test]
    fn connection_detail_scale_stays_visible_for_bytes_per_second_traffic() {
        let rx = vec![0.0, 72.0];
        let tx = vec![0.0, 0.0];
        let combined = vec![0.0, 72.0];
        let scale = sparkline_scale(&rx, &tx, &combined);
        assert!(
            scale < MIN_CHART_SCALE,
            "connection detail scale should not use hero floor: {scale}"
        );
        assert!(scale >= 72.0);
    }

    #[test]
    fn single_sample_history_can_render() {
        let rx = vec![72.0];
        let tx = vec![0.0];
        let combined = vec![72.0];
        let (rx, tx, combined) = pad_series_for_draw(rx, tx, combined);
        assert_eq!(rx.len(), 2);
        assert_eq!(combined.len(), 2);
    }

    #[test]
    fn activity_sparkline_traffic_draws_pixels_in_upper_plot() {
        use crate::chart_test_harness::render_activity_sparkline;
        use crate::theme::Palette;

        let rx = vec![0.0, 30.0, 60.0, 60.0];
        let tx = vec![0.0, 0.0, 0.0, 0.0];
        let combined = vec![0.0, 30.0, 60.0, 60.0];
        let scale = sparkline_scale(&rx, &tx, &combined);
        let palette = Palette::default();

        let chart = render_activity_sparkline(300.0, 56.0, &rx, &tx, &combined, palette, scale);
        let active = chart.count_sparkline_upper_plot_activity(8);
        assert!(
            active > 60,
            "expected visible waveform pixels in upper plot, got {active} (scale={scale})"
        );
    }

    #[test]
    fn inflated_sparkline_scale_hides_waveform_in_upper_plot() {
        use crate::chart_test_harness::render_activity_sparkline;
        use crate::theme::Palette;

        let rx = vec![0.0, 30.0, 60.0, 60.0];
        let tx = vec![0.0, 0.0, 0.0, 0.0];
        let combined = vec![0.0, 30.0, 60.0, 60.0];
        let palette = Palette::default();

        let chart =
            render_activity_sparkline(300.0, 56.0, &rx, &tx, &combined, palette, MIN_CHART_SCALE);
        let active = chart.count_sparkline_upper_plot_activity(8);
        assert!(
            active < 25,
            "inflated scale should flatten waveform into baseline: {active} colored pixels"
        );
    }

    #[test]
    fn activity_sparkline_idle_upper_plot_stays_mostly_background() {
        use crate::chart_test_harness::render_activity_sparkline;
        use crate::theme::Palette;

        let rx = vec![0.0; 32];
        let tx = vec![0.0; 32];
        let combined = vec![0.0; 32];
        let palette = Palette::default();

        let chart = render_activity_sparkline(
            300.0,
            56.0,
            &rx,
            &tx,
            &combined,
            palette,
            MIN_CHART_SCALE,
        );
        let active = chart.count_sparkline_upper_plot_activity(8);
        assert!(
            active < 15,
            "idle sparkline should not paint series in upper plot: {active} pixels"
        );
    }

    #[test]
    fn network_activity_low_traffic_draws_visible_series() {
        use crate::chart_test_harness::render_network_activity;
        use crate::theme::Palette;

        let rx = vec![0.0, 36.0, 72.0, 72.0];
        let tx = vec![0.0, 0.0, 0.0, 0.0];
        let combined = vec![0.0, 36.0, 72.0, 72.0];
        let scale = sparkline_scale(&rx, &tx, &combined);
        let palette = Palette::default();

        let chart = render_network_activity(
            640.0,
            280.0,
            &rx,
            &tx,
            &combined,
            palette,
            TimeWindow::Sec60,
            scale,
        );
        let fill = palette.chart_fill;
        let active = chart.count_pixels_differing_from(60, 20, 620, 240, fill, 10);
        assert!(
            active > 80,
            "hero/detail chart should paint traffic in plot area, got {active} (scale={scale})"
        );
    }

    #[test]
    fn network_activity_draws_grid_lines_in_plot() {
        use crate::chart_test_harness::render_network_activity;
        use crate::theme::Palette;

        let rx = vec![0.0, 7200.0, 4800.0];
        let tx = vec![0.0, 2400.0, 1200.0];
        let combined = vec![0.0, 9600.0, 6000.0];
        let palette = Palette::default();
        let fill = palette.chart_fill;

        let chart = render_network_activity(
            640.0,
            280.0,
            &rx,
            &tx,
            &combined,
            palette,
            TimeWindow::Sec60,
            9600.0,
        );
        let grid = chart.count_pixels_differing_from(60, 130, 620, 150, fill, 8);
        assert!(
            grid > 30,
            "horizontal grid lines should paint in plot area, got {grid}"
        );
    }
}

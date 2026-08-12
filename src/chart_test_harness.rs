//! Off-screen Skia rendering + pixel sampling for chart regression tests.
//!
//! Catches charts that layout correctly but draw invisible waveforms (wrong scale, 1px slivers).

use freya::components::CanvasContext;
use freya::engine::prelude::{raster_n32_premul, Data, FontCollection, Image};
use freya::prelude::{Color, Size2D, TextStyleState};

use crate::charts::{draw_activity_sparkline, draw_network_activity};
use crate::theme::Palette;
use crate::time_window::TimeWindow;

const SPARKLINE_TOP: f32 = 4.0;

/// Owned pixel buffer from an off-screen chart render.
pub struct RenderedChart {
    bytes: Vec<u8>,
    width: i32,
    height: i32,
    row_bytes: usize,
}

impl RenderedChart {
    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    /// RGB at logical pixel (premultiplied source; good enough for diff tests).
    pub fn rgb_at(&self, x: i32, y: i32) -> Option<(u8, u8, u8)> {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return None;
        }
        let offset = y as usize * self.row_bytes + x as usize * 4;
        if offset + 3 >= self.bytes.len() {
            return None;
        }
        Some((
            self.bytes[offset + 2],
            self.bytes[offset + 1],
            self.bytes[offset + 0],
        ))
    }

    pub fn reference_rgb(&self) -> (u8, u8, u8) {
        self.rgb_at(2, 2).unwrap_or((0, 0, 0))
    }

    pub fn max_channel_distance(a: (u8, u8, u8), b: (u8, u8, u8)) -> u8 {
        a.0
            .abs_diff(b.0)
            .max(a.1.abs_diff(b.1))
            .max(a.2.abs_diff(b.2))
    }

    pub fn count_pixels_differing_from(
        &self,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        reference: (u8, u8, u8),
        min_distance: u8,
    ) -> usize {
        let mut count = 0;
        for y in y0..=y1.min(self.height - 1) {
            for x in x0..=x1.min(self.width - 1) {
                if let Some(rgb) = self.rgb_at(x, y) {
                    if Self::max_channel_distance(rgb, reference) >= min_distance {
                        count += 1;
                    }
                }
            }
        }
        count
    }

    /// Upper plot band where traffic waveforms must appear (not the baseline strip).
    pub fn sparkline_upper_plot_region(&self) -> (i32, i32, i32, i32) {
        let floor = self.height - 3;
        let top = SPARKLINE_TOP as i32;
        let bottom = (floor - 8).max(SPARKLINE_TOP as i32 + 4);
        let x0 = (self.width as f32 * 0.45) as i32;
        let x1 = self.width - 3;
        (x0, top, x1, bottom)
    }

    pub fn count_sparkline_upper_plot_activity(&self, min_distance: u8) -> usize {
        let (x0, y0, x1, y1) = self.sparkline_upper_plot_region();
        self.count_pixels_differing_from(x0, y0, x1, y1, self.reference_rgb(), min_distance)
    }
}

pub fn render_with_canvas(width: f32, height: f32, draw: impl FnOnce(&mut CanvasContext<'_>)) -> RenderedChart {
    let mut surface =
        raster_n32_premul((width as i32, height as i32)).expect("raster surface");
    let canvas = surface.canvas();
    let mut font_collection = FontCollection::new();
    let text_style = TextStyleState::default();
    {
        let mut ctx = CanvasContext {
            canvas,
            font_collection: &mut font_collection,
            size: Size2D::new(width, height),
            text_style_state: &text_style,
        };
        draw(&mut ctx);
    }
    let pixmap = surface.peek_pixels().expect("peek_pixels");
    RenderedChart {
        bytes: pixmap.bytes().expect("pixmap bytes").to_vec(),
        width: pixmap.width(),
        height: pixmap.height(),
        row_bytes: pixmap.row_bytes(),
    }
}

/// Render to PNG bytes (README / docs export).
pub fn render_with_canvas_png(
    width: f32,
    height: f32,
    draw: impl FnOnce(&mut CanvasContext<'_>),
) -> Vec<u8> {
    use freya::engine::prelude::EncodedImageFormat;

    let mut surface =
        raster_n32_premul((width as i32, height as i32)).expect("raster surface");
    let canvas = surface.canvas();
    let mut font_collection = FontCollection::new();
    let text_style = TextStyleState::default();
    {
        let mut ctx = CanvasContext {
            canvas,
            font_collection: &mut font_collection,
            size: Size2D::new(width, height),
            text_style_state: &text_style,
        };
        draw(&mut ctx);
    }
    surface
        .image_snapshot()
        .encode(None, EncodedImageFormat::PNG, 100)
        .expect("encode png")
        .as_bytes()
        .to_vec()
}

pub fn render_activity_sparkline(
    width: f32,
    height: f32,
    rx: &[f64],
    tx: &[f64],
    combined: &[f64],
    palette: Palette,
    max_y: f64,
) -> RenderedChart {
    render_with_canvas(width, height, |ctx| {
        draw_activity_sparkline(ctx, rx, tx, combined, palette, max_y);
    })
}

pub fn render_network_activity(
    width: f32,
    height: f32,
    rx: &[f64],
    tx: &[f64],
    combined: &[f64],
    palette: Palette,
    window: TimeWindow,
    max_y: f64,
) -> RenderedChart {
    render_with_canvas(width, height, |ctx| {
        draw_network_activity(ctx, rx, tx, combined, palette, window, max_y);
    })
}

pub fn decode_png_to_chart(png: &[u8]) -> RenderedChart {
    let data = Data::new_copy(png);
    let image = Image::from_encoded(&data).expect("decode png");
    let dims = image.dimensions();
    let width = dims.width;
    let height = dims.height;
    let mut surface = raster_n32_premul((width, height)).expect("raster surface");
    surface.canvas().draw_image(&image, (0, 0), None);
    let pixmap = surface.peek_pixels().expect("peek_pixels");
    RenderedChart {
        bytes: pixmap.bytes().expect("pixmap bytes").to_vec(),
        width: pixmap.width(),
        height: pixmap.height(),
        row_bytes: pixmap.row_bytes(),
    }
}

pub fn palette_track_rgb(palette: Palette) -> (u8, u8, u8) {
    let c: Color = palette.bar_track;
    (c.r(), c.g(), c.b())
}

pub fn palette_chart_fill_rgb(palette: Palette) -> (u8, u8, u8) {
    let (r, g, b) = palette.chart_fill;
    (r, g, b)
}

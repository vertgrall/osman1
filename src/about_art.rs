//! About splash + brand mark — Skia canvas draws (same path as live charts).

use freya::components::CanvasContext;
use freya::engine::prelude::{
    Color4f, FilterMode, MipmapMode, Paint, PaintStyle, PathBuilder, Point, Rect as SkRect,
    RRect, SamplingOptions,
};

use crate::about_assets::{BRAND, SPLASH};

const LOCKUP_W: f32 = 168.;
const LOCKUP_H: f32 = 61.;

pub fn draw_about_splash_card(ctx: &mut CanvasContext) {
    let card_w = ctx.size.width.max(1.0);
    let card_h = ctx.size.height.max(1.0);

    draw_cover_image(ctx, &SPLASH.image, card_w, card_h);

    let lockup_x = card_w - 12. - LOCKUP_W;
    let lockup_y = card_h - 14. - LOCKUP_H;
    draw_lockup_pill(ctx, lockup_x, lockup_y, LOCKUP_W, LOCKUP_H);
    draw_splash_silhouette_in(ctx, lockup_x + 10., lockup_y + 8., 32., 45.);
}

pub fn draw_about_brand_mark(ctx: &mut CanvasContext) {
    let w = ctx.size.width.max(1.0);
    let h = ctx.size.height.max(1.0);
    draw_contain_image(ctx, &BRAND.image, w, h);
}

fn draw_cover_image(
    ctx: &mut CanvasContext,
    img: &freya::engine::prelude::SkImage,
    w: f32,
    h: f32,
) {
    let iw = img.width().max(1) as f32;
    let ih = img.height().max(1) as f32;
    let scale = (w / iw).max(h / ih);
    blit_scaled(ctx, img, iw * scale, ih * scale, w, h);
}

fn draw_contain_image(
    ctx: &mut CanvasContext,
    img: &freya::engine::prelude::SkImage,
    w: f32,
    h: f32,
) {
    let iw = img.width().max(1) as f32;
    let ih = img.height().max(1) as f32;
    let scale = (w / iw).min(h / ih);
    blit_scaled(ctx, img, iw * scale, ih * scale, w, h);
}

fn blit_scaled(
    ctx: &mut CanvasContext,
    img: &freya::engine::prelude::SkImage,
    dw: f32,
    dh: f32,
    w: f32,
    h: f32,
) {
    let dx = (w - dw) * 0.5;
    let dy = (h - dh) * 0.5;
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    let sampling = SamplingOptions::new(FilterMode::Linear, MipmapMode::Linear);
    ctx.canvas.draw_image_rect_with_sampling_options(
        img,
        None,
        SkRect::from_xywh(dx, dy, dw, dh),
        sampling,
        &paint,
    );
}

fn draw_lockup_pill(ctx: &mut CanvasContext, x: f32, y: f32, w: f32, h: f32) {
    let pill = RRect::new_rect_xy(SkRect::from_xywh(x, y, w, h), 8., 8.);
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_style(PaintStyle::Fill);
    paint.set_color4f(Color4f::new(0.0, 0.0, 0.0, 0.42), None);
    ctx.canvas.draw_rrect(pill, &paint);
}

pub fn draw_splash_silhouette_in(ctx: &mut CanvasContext, x: f32, y: f32, w: f32, h: f32) {
    let mut path = PathBuilder::new();

    let pts: [(f32, f32); 38] = [
        (0.84, 0.945),
        (0.93, 0.905),
        (0.97, 0.84),
        (0.93, 0.785),
        (0.975, 0.725),
        (0.92, 0.665),
        (0.96, 0.605),
        (0.90, 0.545),
        (0.94, 0.485),
        (0.86, 0.425),
        (0.91, 0.365),
        (0.82, 0.305),
        (0.88, 0.245),
        (0.76, 0.185),
        (0.82, 0.125),
        (0.68, 0.075),
        (0.58, 0.045),
        (0.48, 0.038),
        (0.40, 0.055),
        (0.34, 0.075),
        (0.28, 0.105),
        (0.22, 0.135),
        (0.18, 0.175),
        (0.14, 0.22),
        (0.10, 0.265),
        (0.065, 0.32),
        (0.035, 0.385),
        (0.018, 0.448),
        (0.045, 0.475),
        (0.075, 0.505),
        (0.095, 0.545),
        (0.10, 0.595),
        (0.115, 0.655),
        (0.14, 0.715),
        (0.20, 0.775),
        (0.32, 0.835),
        (0.48, 0.885),
        (0.66, 0.92),
    ];

    path.move_to(pt(x, y, w, h, pts[0].0, pts[0].1));
    for &(nx, ny) in &pts[1..] {
        path.line_to(pt(x, y, w, h, nx, ny));
    }
    path.close();

    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_style(PaintStyle::Fill);
    paint.set_color4f(Color4f::new(1.0, 1.0, 1.0, 1.0), None);
    ctx.canvas.draw_path(&path.detach(), &paint);
}

fn pt(x: f32, y: f32, w: f32, h: f32, nx: f32, ny: f32) -> Point {
    Point::new(x + nx * w, y + ny * h)
}

#[cfg(test)]
mod tests {
    use freya::components::Canvas;
    use freya::prelude::*;
    use freya_testing::prelude::*;

    use crate::about::{about_content, SPLASH_H, SPLASH_W};
    use crate::theme::Palette;

    #[test]
    fn about_content_uses_canvas_splash_and_brand() {
        let palette = Palette::default();
        let mut test = launch_test(move || {
            rect()
                .width(Size::px(460.))
                .height(Size::px(920.))
                .child(about_content(palette))
        });
        test.sync_and_update();

        let canvases = test.find_many(|node, element| {
            Canvas::try_downcast(element).map(|_| {
                let area = node.layout().area;
                (area.width(), area.height())
            })
        });
        assert!(
            canvases.iter().any(|(w, h)| (*w - SPLASH_W).abs() < 2.0 && (*h - SPLASH_H).abs() < 2.0),
            "expected splash canvas ~300×323, got {canvases:?}"
        );
        assert!(
            canvases.iter().any(|(w, h)| *w >= 80.0 && *w <= 96.0 && *h >= 56.0 && *h <= 72.0),
            "expected brand canvas ~88×64, got {canvases:?}"
        );
    }
}

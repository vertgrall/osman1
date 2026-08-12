//! Off-screen About canvas rendering + pixel checks.
//!
//! Catches About panels that layout correctly but draw blank canvases (missing PNG decode,
//! wrong Skia path, or empty `RenderCallback`).

use crate::about::{SPLASH_H, SPLASH_W};
use crate::about_art::{draw_about_brand_mark, draw_about_splash_card};
use crate::chart_test_harness::{render_with_canvas, render_with_canvas_png, RenderedChart};

const BRAND_W: f32 = 88.;
const BRAND_H: f32 = 64.;

pub fn render_about_splash_card() -> RenderedChart {
    render_with_canvas(SPLASH_W, SPLASH_H, draw_about_splash_card)
}

pub fn render_about_brand_mark() -> RenderedChart {
    render_with_canvas(BRAND_W, BRAND_H, draw_about_brand_mark)
}

/// Minimum pixels in a region that must differ from a flat reference color.
pub fn assert_region_has_visual_content(
    chart: &RenderedChart,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    reference: (u8, u8, u8),
    min_distance: u8,
    min_count: usize,
    label: &str,
) {
    let count = chart.count_pixels_differing_from(x0, y0, x1, y1, reference, min_distance);
    assert!(
        count >= min_count,
        "{label}: expected ≥{min_count} pixels differing from {reference:?} in ({x0},{y0})-({x1},{y1}), got {count}"
    );
}

/// Splash card center should show Tower Village artwork (not a flat empty card).
pub fn assert_splash_card_loads(chart: &RenderedChart) {
    let reference = chart.reference_rgb();
    assert_region_has_visual_content(
        chart,
        40,
        40,
        chart.width() - 40,
        chart.height() - 80,
        reference,
        12,
        800,
        "Tower Village splash center",
    );

    // Lockup pill sits bottom-right; silhouette + pill should darken that band.
    assert_region_has_visual_content(
        chart,
        chart.width() - 120,
        chart.height() - 90,
        chart.width() - 8,
        chart.height() - 8,
        reference,
        18,
        120,
        "New Tower lockup pill region",
    );
}

/// Brand mark canvas should render the embedded PNG, not an empty box.
pub fn assert_brand_mark_loads(chart: &RenderedChart) {
    let reference = chart.reference_rgb();
    assert_region_has_visual_content(
        chart,
        8,
        8,
        chart.width() - 8,
        chart.height() - 8,
        reference,
        10,
        200,
        "New Tower brand mark",
    );
}

#[cfg(test)]
mod tests {
    use freya::components::Canvas;
    use freya::prelude::*;
    use freya_testing::prelude::*;

    use super::*;
    use crate::about::{about_content, about_window};
    use crate::about_assets;
    use crate::theme::Palette;

    fn collect_label_texts(test: &TestingRunner) -> Vec<String> {
        test.find_many(|_, element| {
            Label::try_downcast(element).map(|label| label.text.to_string())
        })
    }

    fn collect_canvas_sizes(test: &TestingRunner) -> Vec<(f32, f32)> {
        test.find_many(|node, element| {
            Canvas::try_downcast(element).map(|_| {
                let area = node.layout().area;
                (area.width(), area.height())
            })
        })
    }

    #[test]
    fn preload_decodes_embedded_about_pngs() {
        about_assets::preload();
        assert!(about_assets::SPLASH.image.width() > 0);
        assert!(about_assets::BRAND.image.width() > 0);
    }

    #[test]
    fn splash_canvas_draws_tower_village_artwork() {
        about_assets::preload();
        let chart = render_about_splash_card();
        assert_eq!(chart.width(), SPLASH_W as i32);
        assert_eq!(chart.height(), SPLASH_H as i32);
        assert_splash_card_loads(&chart);
    }

    #[test]
    fn brand_mark_canvas_draws_embedded_png() {
        about_assets::preload();
        let chart = render_about_brand_mark();
        assert_eq!(chart.width(), BRAND_W as i32);
        assert_eq!(chart.height(), BRAND_H as i32);
        assert_brand_mark_loads(&chart);
    }

    #[test]
    fn about_load_pipeline_assets_to_offscreen_canvas_to_ui() {
        about_assets::preload();

        let splash = render_about_splash_card();
        assert_splash_card_loads(&splash);
        let brand = render_about_brand_mark();
        assert_brand_mark_loads(&brand);

        let mut test = launch_test(|| {
            rect()
                .width(Size::px(460.))
                .height(Size::px(620.))
                .child(about_window())
        });
        test.sync_and_update();

        let labels = collect_label_texts(&test);
        for needle in [
            "About Osman",
            "Osman",
            "By New Tower",
            "See your network breathe.",
            "Designed and Developed by Jon McMillion",
        ] {
            assert!(
                labels.iter().any(|text| text.contains(needle)),
                "about window missing {needle:?}; labels: {labels:?}"
            );
        }

        let canvases = collect_canvas_sizes(&test);
        assert!(
            canvases
                .iter()
                .any(|(w, h)| (*w - SPLASH_W).abs() < 2.0 && (*h - SPLASH_H).abs() < 2.0),
            "about window missing splash canvas; got {canvases:?}"
        );
        assert!(
            canvases.iter().any(|(w, h)| *w >= 80.0 && *w <= 96.0 && *h >= 56.0 && *h <= 72.0),
            "about window missing brand canvas; got {canvases:?}"
        );
    }

    #[test]
    fn about_content_includes_all_branded_sections() {
        about_assets::preload();
        let palette = Palette::default();
        let mut test = launch_test(move || {
            rect()
                .width(Size::px(460.))
                .height(Size::px(920.))
                .child(about_content(palette))
        });
        test.sync_and_update();

        let labels = collect_label_texts(&test);
        for needle in [
            "Osman",
            "By New Tower",
            "Version",
            "Platforms",
            "Overview hero charts",
            "Traffic Character",
            "Process and connection tables",
        ] {
            assert!(
                labels.iter().any(|text| text.contains(needle)),
                "missing {needle:?}; got {labels:?}"
            );
        }

        let canvases = collect_canvas_sizes(&test);
        assert!(canvases.len() >= 2, "expected splash + brand canvases; got {canvases:?}");
    }

    /// Manual docs export: `EXPORT_README=1 cargo test export_readme_screenshots -- --ignored --exact`
    #[test]
    #[ignore = "manual docs export"]
    fn export_readme_screenshots() {
        if std::env::var("EXPORT_README").ok().as_deref() != Some("1") {
            return;
        }

        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs/screenshots");
        std::fs::create_dir_all(&dir).expect("create docs/screenshots");

        about_assets::preload();
        std::fs::write(
            dir.join("about-splash-card.png"),
            render_with_canvas_png(SPLASH_W, SPLASH_H, draw_about_splash_card),
        )
        .expect("write about-splash-card.png");
        std::fs::write(
            dir.join("about-brand-mark.png"),
            render_with_canvas_png(BRAND_W, BRAND_H, draw_about_brand_mark),
        )
        .expect("write about-brand-mark.png");
    }
}

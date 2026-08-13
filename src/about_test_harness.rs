//! Off-screen About canvas rendering + pixel checks.
//!
//! Catches About panels that layout correctly but draw blank canvases (missing PNG decode,
//! wrong Skia path, or empty `RenderCallback`).
//!
//! ## Launch regression policy
//! Any change touching the About code path (window, menubar, settings button, app menu,
//! assets, layout) must extend these checks and pass `./scripts/check-about.sh`.

use crate::about::{SPLASH_H, SPLASH_W};
use crate::about_art::{draw_about_brand_mark, draw_about_splash_card};
use crate::chart_test_harness::{render_with_canvas, render_with_canvas_png, RenderedChart};

const BRAND_W: f32 = 88.;
const BRAND_H: f32 = 64.;

/// Labels that must appear when the About window first opens.
pub const ABOUT_LAUNCH_LABELS: &[&str] = &[
    "Osman",
    "See your network breathe.",
    "By New Tower",
    "Version",
    "Platforms",
    "Designed and Developed by Jon McMillion",
];

/// Splash art must start within this many px of the window top (guards overlay-style drop).
pub const ABOUT_SPLASH_MAX_Y: f32 = 80.;

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

/// Headless launch snapshot — used by `about_contract` and UI tests.
#[cfg(test)]
pub mod launch_checks {
    use freya::components::Canvas;
    use freya::elements::image::Image;
    use freya::prelude::*;
    use freya_testing::prelude::*;

    use super::{ABOUT_LAUNCH_LABELS, ABOUT_SPLASH_MAX_Y};
    use crate::about::{about_content, about_window, ABOUT_WINDOW_H, ABOUT_WINDOW_W, SPLASH_H, SPLASH_W};
    use crate::about_assets;
    use crate::theme::AppTheme;

    #[derive(Debug)]
    pub struct AboutLaunchSnapshot {
        pub labels: Vec<String>,
        pub splash_y: f32,
        pub splash_size: (f32, f32),
        pub brand_count: usize,
    }

    fn collect_label_texts(test: &TestingRunner) -> Vec<String> {
        test.find_many(|_, element| {
            Label::try_downcast(element).map(|label| label.text.to_string())
        })
    }

    fn splash_canvas_snapshot(test: &TestingRunner) -> (f32, f32, f32) {
        test.find_many(|node, element| {
            Canvas::try_downcast(element).and_then(|_| {
                let area = node.layout().area;
                ((area.width() - SPLASH_W).abs() < 4.0 && (area.height() - SPLASH_H).abs() < 4.0)
                    .then_some((area.origin.y, area.width(), area.height()))
            })
        })
        .into_iter()
        .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
        .unwrap_or((f32::MAX, 0., 0.))
    }

    fn count_brand_assets(test: &TestingRunner) -> usize {
        let canvases = test.find_many(|node, element| {
            Canvas::try_downcast(element).and_then(|_| {
                let area = node.layout().area;
                (area.width() >= 80.0
                    && area.width() <= 96.0
                    && area.height() >= 56.0
                    && area.height() <= 72.0)
                    .then_some(())
            })
        });
        let images = test.find_many(|node, element| {
            Image::try_downcast(element).and_then(|_| {
                let area = node.layout().area;
                (area.width() >= 80.0
                    && area.width() <= 96.0
                    && area.height() >= 56.0
                    && area.height() <= 72.0)
                    .then_some(())
            })
        });
        canvases.len() + images.len()
    }

    pub fn snapshot_about_window_launch() -> AboutLaunchSnapshot {
        about_assets::preload();

        let mut test = TestingRunner::new(
            about_window,
            Size2D::new(ABOUT_WINDOW_W, ABOUT_WINDOW_H),
            |_| {},
            1.0,
        )
        .0;
        test.sync_and_update();

        let (splash_y, splash_w, splash_h) = splash_canvas_snapshot(&test);
        AboutLaunchSnapshot {
            labels: collect_label_texts(&test),
            splash_y,
            splash_size: (splash_w, splash_h),
            brand_count: count_brand_assets(&test),
        }
    }

    pub fn assert_about_window_launches_ok(snapshot: &AboutLaunchSnapshot) {
        for needle in ABOUT_LAUNCH_LABELS {
            assert!(
                snapshot.labels.iter().any(|text| text.contains(needle)),
                "About launch missing label containing {needle:?}; got {:?}",
                snapshot.labels
            );
        }

        assert!(
            snapshot.splash_y < ABOUT_SPLASH_MAX_Y,
            "About splash must render near top on launch; splash_y={} max={}",
            snapshot.splash_y,
            ABOUT_SPLASH_MAX_Y
        );
        assert!(
            (snapshot.splash_size.0 - SPLASH_W).abs() < 4.0
                && (snapshot.splash_size.1 - SPLASH_H).abs() < 4.0,
            "About splash canvas wrong size; got {:?}",
            snapshot.splash_size
        );
        assert!(
            snapshot.brand_count >= 1,
            "About launch missing New Tower brand mark canvas"
        );
    }

    pub fn assert_about_content_renders_for_theme(theme: AppTheme) {
        about_assets::preload();
        let palette = theme.palette();
        let mut test = launch_test(move || {
            rect()
                .vertical()
                .width(Size::px(ABOUT_WINDOW_W))
                .main_align(Alignment::Start)
                .child(about_content(palette))
        });
        test.sync_and_update();

        let (splash_y, _, _) = splash_canvas_snapshot(&test);
        assert!(
            splash_y < ABOUT_SPLASH_MAX_Y,
            "{theme:?} theme: splash dropped on launch layout; splash_y={splash_y}"
        );

        let labels = collect_label_texts(&test);
        assert!(
            labels.iter().any(|text| text == "Osman"),
            "{theme:?} theme missing Osman title; labels={labels:?}"
        );
    }

    pub fn assert_window_constants_match_menubar_config() {
        let menubar_rs = std::fs::read_to_string("src/menubar.rs").expect("read src/menubar.rs");
        assert!(
            menubar_rs.contains("WindowConfig::new(about_window)"),
            "menubar must launch about_window component"
        );
        assert!(
            menubar_rs.contains(".with_size(ABOUT_WINDOW_W as f64, ABOUT_WINDOW_H as f64)"),
            "menubar must size About window from shared constants"
        );
        assert!(
            menubar_rs.contains(".with_max_size(ABOUT_WINDOW_W as f64, ABOUT_WINDOW_H as f64)"),
            "menubar must cap About window size"
        );
        assert!(
            menubar_rs.contains(".with_resizable(false)"),
            "About window must stay fixed-size"
        );
    }
}

#[cfg(test)]
mod tests {
    use freya::components::Canvas;
    use freya::elements::image::Image;
    use freya::prelude::*;
    use freya_testing::prelude::*;

    use super::*;
    use crate::about::{about_content, about_window};
    use crate::about_assets;
    use crate::theme::{AppTheme, Palette};

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

    fn collect_image_sizes(test: &TestingRunner) -> Vec<(f32, f32)> {
        test.find_many(|node, element| {
            Image::try_downcast(element).map(|_| {
                let area = node.layout().area;
                (area.width(), area.height())
            })
        })
    }

    fn assert_branded_assets_present(test: &TestingRunner) {
        let canvases = collect_canvas_sizes(test);
        let images = collect_image_sizes(test);
        assert!(
            images
                .iter()
                .any(|(w, h)| (*w - SPLASH_W).abs() < 2.0 && (*h - SPLASH_H).abs() < 2.0)
                || canvases
                    .iter()
                    .any(|(w, h)| (*w - SPLASH_W).abs() < 2.0 && (*h - SPLASH_H).abs() < 2.0),
            "missing splash asset; images={images:?} canvases={canvases:?}"
        );
        assert!(
            images
                .iter()
                .any(|(w, h)| *w >= 80.0 && *w <= 96.0 && *h >= 56.0 && *h <= 72.0)
                || canvases.iter().any(|(w, h)| {
                    *w >= 80.0 && *w <= 96.0 && *h >= 56.0 && *h <= 72.0
                }),
            "missing brand asset; images={images:?} canvases={canvases:?}"
        );
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
    fn about_window_launch_regression() {
        use super::launch_checks::{
            assert_about_window_launches_ok, snapshot_about_window_launch,
        };

        let snapshot = snapshot_about_window_launch();
        assert_about_window_launches_ok(&snapshot);
    }

    #[test]
    fn about_content_renders_for_every_theme_without_splash_drop() {
        use super::launch_checks::assert_about_content_renders_for_theme;

        for theme in AppTheme::ALL {
            assert_about_content_renders_for_theme(theme);
        }
    }

    #[test]
    fn about_window_config_matches_shared_constants() {
        use super::launch_checks::assert_window_constants_match_menubar_config;

        assert_window_constants_match_menubar_config();
    }

    #[test]
    fn about_load_pipeline_assets_to_offscreen_canvas_to_ui() {
        about_assets::preload();

        let splash = render_about_splash_card();
        assert_splash_card_loads(&splash);
        let brand = render_about_brand_mark();
        assert_brand_mark_loads(&brand);

        use super::launch_checks::{
            assert_about_window_launches_ok, snapshot_about_window_launch,
        };

        assert_about_window_launches_ok(&snapshot_about_window_launch());
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

        assert_branded_assets_present(&test);
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

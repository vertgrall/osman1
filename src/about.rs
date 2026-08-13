//! About panel — small dedicated window with Tower Village splash + copy.
//!
//! ## Invariants (enforced by `about_contract` tests — do not break)
//! 1. Open About via **`menubar::request_about_window`** → small secondary window.
//! 2. **Never** mount About as a main-window overlay (viewport clipping regressions).
//! 3. Splash + brand mark render with **canvas + preloaded Skia PNGs**, not `image()` / `ImageViewer`.
//! 4. Settings shows **“About Osman…”** only — do not embed `about_content` inline.
//! 5. macOS **Osman → About** must call `request_about_window()` (not the system panel).
//! 6. Call **`about_assets::preload()`** before first About paint (`main` already does).
//! 7. **Regression policy:** any change touching this path → extend `about_contract` /
//!    `about_test_harness::launch_checks` and run `./scripts/check-about.sh`.
//!
//! Quick check: `./scripts/check-about.sh`

use freya::prelude::*;

use crate::about_art::{draw_about_brand_mark, draw_about_splash_card};
use crate::about_assets::preload;
use crate::preferences;
use crate::theme::Palette;

mod git_metadata {
    include!(concat!(env!("OUT_DIR"), "/git_metadata.rs"));
}

const APP_NAME: &str = "Osman";
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const APP_BUILD: &str = "1";

pub const SPLASH_W: f32 = 300.;
pub const SPLASH_H: f32 = 323.;

pub fn about_content(palette: Palette) -> Element {
    rect()
        .vertical()
        .width(Size::px(420.))
        .main_align(Alignment::Start)
        .cross_align(Alignment::Center)
        .spacing(16.)
        .child(about_splash_header(palette))
        .child(about_title_block(palette))
        .child(about_bullets(palette))
        .child(about_brand_mark())
        .child(about_meta_rows(palette))
        .child(
            label()
                .text("Designed and Developed by Jon McMillion")
                .font_size(11.)
                .color(palette.muted)
                .text_align(TextAlign::Center)
                .width(Size::px(380.)),
        )
        .into()
}

pub const ABOUT_WINDOW_W: f32 = 440.;
pub const ABOUT_WINDOW_H: f32 = 640.;

/// Root component for the small About window.
pub fn about_window() -> Element {
    preload();
    preferences::ensure_init();
    let palette = preferences::get().app_theme().palette();

    rect()
        .expanded()
        .background(palette.bg)
        .padding(Gaps::new_all(16.))
        .child(ScrollView::new().expanded().child(about_content(palette)))
        .into()
}

fn about_splash_header(palette: Palette) -> Element {
    rect()
        .width(Size::px(SPLASH_W))
        .height(Size::px(SPLASH_H))
        .corner_radius(10.)
        .border(palette.border())
        .overflow(Overflow::Clip)
        .child(
            canvas(RenderCallback::new(|ctx| draw_about_splash_card(ctx)))
                .width(Size::px(SPLASH_W))
                .height(Size::px(SPLASH_H)),
        )
        .child(splash_lockup_label())
        .into()
}

fn splash_lockup_label() -> Element {
    rect()
        .position(Position::new_absolute().bottom(22.).right(24.))
        .padding(Gaps::new(0., 0., 0., 42.))
        .child(
            label()
                .text("By New Tower")
                .font_size(14.)
                .font_family("Times New Roman")
                .color(Color::from_rgb(255, 255, 255)),
        )
        .into()
}

fn about_brand_mark() -> Element {
    canvas(RenderCallback::new(|ctx| draw_about_brand_mark(ctx)))
        .width(Size::px(88.))
        .height(Size::px(64.))
        .into()
}

fn about_title_block(palette: Palette) -> Element {
    rect()
        .vertical()
        .spacing(8.)
        .width(Size::px(400.))
        .cross_align(Alignment::Center)
        .child(
            label()
                .text(APP_NAME)
                .font_size(22.)
                .font_weight(FontWeight::BOLD)
                .color(palette.title)
                .text_align(TextAlign::Center),
        )
        .child(
            label()
                .text("See your network breathe.")
                .font_size(15.)
                .font_weight(FontWeight::BOLD)
                .color(palette.text)
                .text_align(TextAlign::Center),
        )
        .child(
            label()
                .text(
                    "Native network traffic monitor for Mac — live adapters, traffic character scopes, \
                     process drill-down, alerts, and a menu bar mini monitor.",
                )
                .font_size(13.)
                .color(palette.muted)
                .text_align(TextAlign::Center)
                .width(Size::px(400.)),
        )
        .into()
}

fn about_bullets(palette: Palette) -> Element {
    rect()
        .vertical()
        .spacing(6.)
        .width(Size::px(400.))
        .padding(Gaps::new(0., 4., 0., 4.))
        .children([
            about_bullet(
                "Overview hero charts and adapter sparklines with clinical scopes",
                palette,
            ),
            about_bullet(
                "Traffic Character classifies live patterns · alerts when thresholds spike",
                palette,
            ),
            about_bullet(
                "Process and connection tables · menubar live rates without leaving your flow",
                palette,
            ),
        ])
        .into()
}

fn about_bullet(text: &'static str, palette: Palette) -> Element {
    rect()
        .horizontal()
        .spacing(8.)
        .width(Size::fill())
        .child(
            label()
                .text("·")
                .font_size(13.)
                .font_weight(FontWeight::BOLD)
                .color(palette.muted),
        )
        .child(
            label()
                .text(text)
                .font_size(13.)
                .color(palette.muted)
                .width(Size::px(380.)),
        )
        .into()
}

fn about_meta_rows(palette: Palette) -> Element {
    rect()
        .vertical()
        .spacing(8.)
        .width(Size::px(400.))
        .children(meta_rows(palette))
        .into()
}

fn meta_rows(palette: Palette) -> Vec<Element> {
    let mut rows = vec![meta_row(
        "Version",
        format!("{APP_VERSION} ({APP_BUILD})"),
        palette,
        false,
    )];

    if git_metadata::BRANCH != "unknown" && git_metadata::SHORT_HASH != "unknown" {
        rows.push(meta_row(
            "Git",
            format!("{} · {}", git_metadata::BRANCH, git_metadata::SHORT_HASH),
            palette,
            true,
        ));
    }

    rows.push(meta_row(
        "Platforms",
        "macOS 14+".to_string(),
        palette,
        false,
    ));
    rows
}

fn meta_row(label_text: &'static str, value: String, palette: Palette, mono: bool) -> Element {
    let mut value_label = label()
        .text(value)
        .font_size(if mono { 11. } else { 13. })
        .color(palette.muted);

    if mono {
        value_label = value_label.font_family("Menlo");
    }

    rect()
        .horizontal()
        .width(Size::fill())
        .child(
            label()
                .text(label_text)
                .font_size(13.)
                .color(palette.text),
        )
        .child(rect().width(Size::fill()).child(value_label))
        .into()
}

#[cfg(test)]
mod tests {
    use freya::components::Canvas;
    use freya::elements::image::Image;
    use freya::prelude::*;
    use freya_testing::prelude::*;

    use super::{about_content, about_window, ABOUT_WINDOW_H, ABOUT_WINDOW_W, SPLASH_H, SPLASH_W};
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

    fn collect_image_sizes(test: &TestingRunner) -> Vec<(f32, f32)> {
        test.find_many(|node, element| {
            Image::try_downcast(element).map(|_| {
                let area = node.layout().area;
                (area.width(), area.height())
            })
        })
    }

    #[test]
    fn about_content_renders_new_tower_branded_layout() {
        let palette = Palette::default();

        fn app(palette: Palette) -> impl IntoElement {
            rect()
                .width(Size::px(460.))
                .height(Size::px(920.))
                .padding(Gaps::new_all(16.))
                .child(about_content(palette))
        }

        let mut test = launch_test(move || app(palette));
        test.sync_and_update();

        let labels = collect_label_texts(&test);
        for needle in [
            "Osman",
            "See your network breathe.",
            "By New Tower",
            "Version",
            "Platforms",
            "Designed and Developed by Jon McMillion",
            "Overview hero charts and adapter sparklines with clinical scopes",
        ] {
            assert!(
                labels.iter().any(|text| text.contains(needle)),
                "missing label containing {needle:?}; got: {labels:?}"
            );
        }

        let canvases = collect_canvas_sizes(&test);
        let images = collect_image_sizes(&test);
        assert!(
            images
                .iter()
                .any(|(w, h)| (*w - SPLASH_W).abs() < 2.0 && (*h - SPLASH_H).abs() < 2.0)
                || canvases
                    .iter()
                    .any(|(w, h)| (*w - SPLASH_W).abs() < 2.0 && (*h - SPLASH_H).abs() < 2.0),
            "Tower Village splash missing; images={images:?} canvases={canvases:?}"
        );
        assert!(
            images
                .iter()
                .any(|(w, h)| *w >= 80.0 && *w <= 96.0 && *h >= 56.0 && *h <= 72.0)
                || canvases.iter().any(|(w, h)| {
                    *w >= 80.0 && *w <= 96.0 && *h >= 56.0 && *h <= 72.0
                }),
            "New Tower brand mark missing; images={images:?} canvases={canvases:?}"
        );
    }

    #[test]
    fn about_window_renders_branded_canvases_and_copy() {
        use crate::about_test_harness::launch_checks::{
            assert_about_window_launches_ok, snapshot_about_window_launch,
        };

        assert_about_window_launches_ok(&snapshot_about_window_launch());
    }

    #[test]
    fn about_window_export_visual_regression() {
        use freya::prelude::Size2D;

        let mut test = TestingRunner::new(
            about_window,
            Size2D::new(ABOUT_WINDOW_W, ABOUT_WINDOW_H),
            |_| {},
            1.0,
        )
        .0;
        test.sync_and_update();

        let path = std::env::temp_dir().join("osman-about-window-regression.png");
        test.render_to_file(&path);

        let labels = collect_label_texts(&test);
        assert!(labels.iter().any(|t| t == "Osman"));
        assert!(labels.iter().any(|t| t.contains("By New Tower")));
        assert!(path.exists(), "expected screenshot at {}", path.display());
    }

    #[test]
    fn about_content_layout_without_modal_shell() {
        let palette = Palette::default();
        let mut test = launch_test(move || {
            rect()
                .vertical()
                .width(Size::px(480.))
                .height(Size::px(900.))
                .main_align(Alignment::Start)
                .child(about_content(palette))
        });
        test.sync_and_update();

        let splash_y = test
            .find_many(|node, element| {
                Canvas::try_downcast(element).and_then(|_| {
                    let area = node.layout().area;
                    ((area.width() - SPLASH_W).abs() < 4.0 && (area.height() - SPLASH_H).abs() < 4.0)
                        .then_some(area.origin.y)
                })
            })
            .into_iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .expect("splash canvas missing");

        assert!(
            splash_y < 80.0,
            "about_content alone should pin splash to top; splash_y={splash_y}"
        );
    }

    #[test]
    fn settings_about_button_uses_request_about_window() {
        use crate::menubar::{request_about_window, set_renderer_dispatch_for_test};

        let ran = std::sync::Arc::new(std::sync::Mutex::new(false));
        let ran_cb = ran.clone();
        set_renderer_dispatch_for_test(Box::new(move |cb| {
            *ran_cb.lock().expect("lock") = true;
            let _ = cb;
        }));

        request_about_window();

        assert!(
            *ran.lock().expect("lock"),
            "Settings About path must call request_about_window"
        );
    }
}

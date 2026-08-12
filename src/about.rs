//! About panel — layout mirrors Mohawk's `AboutMohawkContent`.

use freya::prelude::*;

use crate::about_art::{draw_about_brand_mark, draw_about_splash_card};
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

pub fn about_window() -> Element {
    let palette = Palette::default();

    rect()
        .expanded()
        .background(palette.bg)
        .vertical()
        .padding(Gaps::new_all(20.))
        .spacing(12.)
        .child(
            label()
                .text(format!("About {APP_NAME}"))
                .font_size(18.)
                .font_weight(FontWeight::BOLD)
                .color(palette.title),
        )
        .child(
            ScrollView::new()
                .expanded()
                .child(
                    rect()
                        .vertical()
                        .width(Size::fill())
                        .cross_align(Alignment::Center)
                        .padding(Gaps::new(0., 0., 12., 0.))
                        .child(about_content(palette)),
                ),
        )
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
        .child(rect().expanded().child(value_label))
        .into()
}

#[cfg(test)]
mod tests {
    use freya::components::Canvas;
    use freya::prelude::*;
    use freya_testing::prelude::*;

    use super::{about_content, about_window, SPLASH_H, SPLASH_W};
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
        assert!(
            canvases
                .iter()
                .any(|(w, h)| (*w - SPLASH_W).abs() < 2.0 && (*h - SPLASH_H).abs() < 2.0),
            "Tower Village splash canvas missing; got {canvases:?}"
        );
        assert!(
            canvases
                .iter()
                .any(|(w, h)| *w >= 80.0 && *w <= 96.0 && *h >= 56.0 && *h <= 72.0),
            "New Tower brand mark canvas missing; got {canvases:?}"
        );
    }

    #[test]
    fn about_window_renders_title_and_branded_canvases() {
        fn app() -> impl IntoElement {
            rect()
                .width(Size::px(460.))
                .height(Size::px(620.))
                .child(about_window())
        }

        let mut test = launch_test(app);
        test.sync_and_update();

        let labels = collect_label_texts(&test);
        assert!(
            labels.iter().any(|text| text.contains("About Osman")),
            "About window title missing; got: {labels:?}"
        );
        assert!(
            labels.iter().any(|text| text.contains("By New Tower")),
            "About window body missing lockup; got: {labels:?}"
        );

        let canvases = collect_canvas_sizes(&test);
        assert!(
            canvases.len() >= 2,
            "About window should render splash + brand canvases; got {canvases:?}"
        );
    }

    #[test]
    fn settings_panel_embeds_new_tower_about_content() {
        use crate::about::about_content;

        let palette = Palette::default();
        let mut test = launch_test(move || {
            rect()
                .width(Size::px(520.))
                .height(Size::px(980.))
                .padding(Gaps::new_all(16.))
                .child(about_content(palette))
        });
        test.sync_and_update();

        let labels = collect_label_texts(&test);
        assert!(
            labels.iter().any(|t| t.contains("By New Tower")),
            "settings about must include New Tower lockup; got {labels:?}"
        );
        let canvases = collect_canvas_sizes(&test);
        assert!(
            canvases.iter().any(|(w, h)| (*w - SPLASH_W).abs() < 2.0),
            "settings about must include splash canvas; got {canvases:?}"
        );
    }
}

//! First-run onboarding — privacy copy and dismiss sheet.

use std::cell::RefCell;
use std::rc::Rc;

use freya::prelude::*;

use crate::theme::Palette;

pub const TITLE: &str = "Welcome to Osman";
pub const SUBTITLE: &str = "Network traffic monitor for macOS · New Tower";

pub const READS: &[&str] = &[
    "Adapter receive and send rates via sysinfo",
    "Process and connection details via macOS nettop and lsof",
    "Live menubar rates while you work",
];

pub const DOES_NOT: &[&str] = &[
    "Capture packets (no PCAP) or inspect payload contents",
    "Require root or install kernel extensions",
    "Upload your traffic — everything stays on this Mac",
];

fn dismiss(mut visible: State<bool>, on_dismiss: Rc<RefCell<dyn FnMut()>>) {
    on_dismiss.borrow_mut()();
    visible.set(false);
}

/// Full-window scrim + centered sheet. Empty element when not visible.
pub fn onboarding_overlay(
    palette: Palette,
    visible: State<bool>,
    on_dismiss: impl FnMut() + 'static,
    mut on_open_settings: impl FnMut() + 'static,
    mut on_open_about: impl FnMut() + 'static,
) -> Element {
    if !*visible.read() {
        return rect().into();
    }

    let on_dismiss = Rc::new(RefCell::new(on_dismiss));
    let dismiss_started = on_dismiss.clone();
    let dismiss_settings = on_dismiss.clone();
    let dismiss_about = on_dismiss;
    let visible_started = visible;
    let visible_settings = visible;
    let visible_about = visible;

    rect()
        .expanded()
        .position(Position::new_absolute().top(0.).left(0.).right(0.).bottom(0.))
        .layer(Layer::Overlay)
        .background(Color::from_argb(140, 45, 41, 36))
        .main_align(Alignment::Center)
        .cross_align(Alignment::Center)
        .child(onboarding_sheet(
            palette,
            move || dismiss(visible_started, dismiss_started.clone()),
            move || {
                on_open_settings();
                dismiss(visible_settings, dismiss_settings.clone());
            },
            move || {
                on_open_about();
                dismiss(visible_about, dismiss_about.clone());
            },
        ))
        .into()
}

pub fn onboarding_sheet(
    palette: Palette,
    mut on_get_started: impl FnMut() + 'static,
    mut on_settings: impl FnMut() + 'static,
    mut on_about: impl FnMut() + 'static,
) -> Element {
    rect()
        .width(Size::px(480.))
        .background(palette.panel)
        .corner_radius(12.)
        .border(palette.border())
        .padding(Gaps::new_all(20.))
        .spacing(14.)
        .child(
            label()
                .text(TITLE)
                .font_size(20.)
                .font_weight(FontWeight::BOLD)
                .color(palette.title),
        )
        .child(
            label()
                .text(SUBTITLE)
                .font_size(13.)
                .color(palette.muted),
        )
        .child(onboarding_section("What Osman reads", READS, palette))
        .child(onboarding_section("What Osman does not do", DOES_NOT, palette))
        .child(
            rect()
                .horizontal()
                .spacing(10.)
                .main_align(Alignment::End)
                .width(Size::fill())
                .child(onboarding_primary_button("Get started", palette, move |_| on_get_started()))
                .child(onboarding_text_button("Settings", palette, move |_| on_settings()))
                .child(onboarding_text_button("About", palette, move |_| on_about())),
        )
        .into()
}

fn onboarding_section(title: &str, bullets: &[&str], palette: Palette) -> Element {
    let title = title.to_string();
    rect()
        .vertical()
        .spacing(6.)
        .width(Size::fill())
        .child(
            label()
                .text(title)
                .font_size(13.)
                .font_weight(FontWeight::BOLD)
                .color(palette.text),
        )
        .children(bullets.iter().map(|text| onboarding_bullet(text, palette)))
        .into()
}

fn onboarding_bullet(text: &str, palette: Palette) -> Element {
    let text = text.to_string();
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
                .font_size(12.)
                .color(palette.muted),
        )
        .into()
}

fn onboarding_primary_button(
    label_text: &str,
    palette: Palette,
    on_press: impl FnMut(Event<MouseEventData>) + 'static,
) -> Element {
    rect()
        .padding(Gaps::new(10., 16., 10., 16.))
        .background(palette.accent)
        .corner_radius(8.)
        .on_mouse_up(on_press)
        .child(
            label()
                .text(label_text.to_string())
                .font_size(13.)
                .font_weight(FontWeight::BOLD)
                .color(Color::from_rgb(255, 255, 255)),
        )
        .into()
}

fn onboarding_text_button(
    label_text: &str,
    palette: Palette,
    on_press: impl FnMut(Event<MouseEventData>) + 'static,
) -> Element {
    rect()
        .padding(Gaps::new(10., 12., 10., 12.))
        .corner_radius(8.)
        .on_mouse_up(on_press)
        .child(
            label()
                .text(label_text.to_string())
                .font_size(13.)
                .color(palette.receive),
        )
        .into()
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    use freya::prelude::*;
    use freya_testing::prelude::*;

    use super::*;
    use crate::theme::Palette;

    #[test]
    fn onboarding_sheet_renders_welcome_copy() {
        let palette = Palette::default();
        let mut test = launch_test({
            move || onboarding_sheet(palette, || {}, || {}, || {})
        });
        test.sync_and_update();

        let labels: Vec<String> = test.find_many(|_, element| {
            Label::try_downcast(element).map(|label| label.text.to_string())
        });
        assert!(
            labels.iter().any(|text| text.contains(TITLE)),
            "expected title, got {labels:?}"
        );
        assert!(
            labels.iter().any(|text| text.contains("no PCAP")),
            "expected privacy bullet, got {labels:?}"
        );
    }

    #[test]
    fn get_started_dismisses_overlay() {
        let palette = Palette::default();
        let dismissed = Arc::new(AtomicBool::new(false));

        let mut test = launch_test({
            let dismissed = dismissed.clone();
            move || {
                let visible = use_state(|| true);
                let dismissed = dismissed.clone();
                onboarding_overlay(
                    palette,
                    visible,
                    move || {
                        dismissed.store(true, Ordering::SeqCst);
                    },
                    || {},
                    || {},
                )
            }
        });
        test.sync_and_update();

        let mut targets = test.find_many(|node, element| {
            Label::try_downcast(element).and_then(|label| {
                (label.text == "Get started").then(|| {
                    let area = node.layout().area;
                    (
                        area.origin.x,
                        (
                            (area.origin.x + area.width() / 2.) as f64,
                            (area.origin.y + area.height() / 2.) as f64,
                        ),
                    )
                })
            })
        });
        targets.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let (_, target) = targets.first().expect("Get started button");
        test.click_cursor(*target);
        test.sync_and_update();

        assert!(dismissed.load(Ordering::SeqCst));
    }
}

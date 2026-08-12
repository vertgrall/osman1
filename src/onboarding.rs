//! First-run onboarding — privacy copy and one-time dismiss flag.

use std::fs;
use std::io;
use std::path::PathBuf;

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

/// Tracks whether the user completed first-run onboarding.
#[derive(Clone, Debug)]
pub struct OnboardingStore {
    flag_path: PathBuf,
}

impl OnboardingStore {
    pub fn production() -> Self {
        Self {
            flag_path: production_flag_path(),
        }
    }

    pub fn at(flag_path: PathBuf) -> Self {
        Self { flag_path }
    }

    pub fn flag_path(&self) -> &PathBuf {
        &self.flag_path
    }

    pub fn has_seen(&self) -> bool {
        self.flag_path.exists()
    }

    pub fn mark_seen(&self) -> io::Result<()> {
        if let Some(parent) = self.flag_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&self.flag_path, "1")
    }
}

fn production_flag_path() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home).join("Library/Application Support/Osman/onboarding_done")
    } else {
        PathBuf::from(".local/share/Osman/onboarding_done")
    }
}

fn dismiss(store: &OnboardingStore, mut visible: State<bool>) {
    let _ = store.mark_seen();
    visible.set(false);
}

/// Full-window scrim + centered sheet. Empty element when not visible.
pub fn onboarding_overlay(
    palette: Palette,
    visible: State<bool>,
    store: OnboardingStore,
    mut on_open_settings: impl FnMut() + 'static,
    mut on_open_about: impl FnMut() + 'static,
) -> Element {
    if !*visible.read() {
        return rect().into();
    }

    let store_started = store.clone();
    let store_settings = store.clone();
    let store_about = store;
    let visible_started = visible;
    let visible_settings = visible;
    let visible_about = visible;

    rect()
        .expanded()
        .position(Position::new_absolute().top(0.).left(0.).right(0.).bottom(0.))
        .background(Color::from_argb(140, 45, 41, 36))
        .main_align(Alignment::Center)
        .cross_align(Alignment::Center)
        .child(onboarding_sheet(
            palette,
            move || dismiss(&store_started, visible_started),
            move || {
                on_open_settings();
                dismiss(&store_settings, visible_settings);
            },
            move || {
                on_open_about();
                dismiss(&store_about, visible_about);
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
    use freya::prelude::*;
    use freya_testing::prelude::*;

    use super::*;
    use crate::theme::Palette;

    fn temp_store() -> OnboardingStore {
        let path = std::env::temp_dir().join(format!(
            "osman-onboarding-{}",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        OnboardingStore::at(path)
    }

    #[test]
    fn has_seen_false_when_missing() {
        let store = temp_store();
        assert!(!store.has_seen());
    }

    #[test]
    fn mark_seen_creates_flag_file() {
        let store = temp_store();
        store.mark_seen().expect("mark seen");
        assert!(store.has_seen());
        assert!(store.flag_path().is_file());
        let _ = fs::remove_file(store.flag_path());
    }

    #[test]
    fn onboarding_sheet_renders_welcome_copy() {
        let palette = Palette::default();
        let mut test = launch_test({
            move || {
                onboarding_sheet(
                    palette,
                    || {},
                    || {},
                    || {},
                )
            }
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
        assert!(
            labels.iter().any(|text| text == "Get started"),
            "expected primary button, got {labels:?}"
        );
    }

    #[test]
    fn get_started_dismisses_overlay() {
        let palette = Palette::default();
        let store = temp_store();
        let store_for_ui = store.clone();

        let mut test = launch_test({
            let store_for_ui = store_for_ui.clone();
            move || {
                let visible = use_state(|| true);
                onboarding_overlay(
                    palette,
                    visible,
                    store_for_ui.clone(),
                    || {},
                    || {},
                )
            }
        });
        test.sync_and_update();

        let before: Vec<String> = test.find_many(|_, element| {
            Label::try_downcast(element).map(|label| label.text.to_string())
        });
        assert!(before.iter().any(|text| text.contains(TITLE)));

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
        let (_, target) = targets
            .first()
            .expect("Get started button");
        test.click_cursor(*target);
        test.sync_and_update();

        assert!(store.has_seen());
        let after: Vec<String> = test.find_many(|_, element| {
            Label::try_downcast(element).map(|label| label.text.to_string())
        });
        assert!(
            !after.iter().any(|text| text.contains(TITLE)),
            "overlay should hide after dismiss, still showing {after:?}"
        );
        let _ = fs::remove_file(store.flag_path());
    }
}

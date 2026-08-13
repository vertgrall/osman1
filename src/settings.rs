//! Settings screen — General · Alerts · About sections.
//!
//! All `use_state` hooks for this screen live in `app_with_bootstrap` (`main.rs`) so
//! Freya never sees hooks mounted conditionally when the user opens Settings.

use freya::prelude::*;

use crate::alerts::{alerts_screen, AlertEngine};
use crate::menubar;
use crate::preferences;
use crate::theme::{AppTheme, Palette};

use crate::macos_activation;

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

const POLL_OPTIONS: &[(u64, &str)] = &[(500, "0.5 s"), (1000, "1 s"), (2000, "2 s")];

const DEFAULT_SECTIONS: &[(&str, &str)] = &[
    ("overview", "Overview"),
    ("adapters", "Adapters"),
    ("processes", "Processes"),
    ("connections", "Connections"),
    ("traffic_character", "Traffic Character"),
    ("alerts", "Alerts"),
    ("settings", "Settings"),
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    General,
    Alerts,
    About,
}

pub fn initial_settings_tab() -> SettingsTab {
    SettingsTab::General
}

pub fn initial_poll_ms() -> u64 {
    preferences::get().normalized_poll_ms()
}

pub fn initial_default_section() -> String {
    preferences::get().default_section.clone()
}

pub fn initial_menubar_only() -> bool {
    preferences::get().menubar_only
}

pub fn settings_screen(
    palette: Palette,
    app_theme: State<AppTheme>,
    alerts: &AlertEngine,
    active: bool,
    active_tab: State<SettingsTab>,
    poll_ms: State<u64>,
    default_section: State<String>,
    menubar_only: State<bool>,
) -> Element {
    let tab = *active_tab.read();
    let poll = *poll_ms.read();
    let section_id = default_section.read().clone();
    let menubar = *menubar_only.read();

    let mut root = ScrollView::new().expanded();
    if !active {
        root = root.height(Size::px(0.));
    }

    root.child(
        rect()
            .vertical()
            .width(Size::fill())
            .spacing(14.)
            .child(settings_tab_bar(tab, palette, active_tab))
            .child(match tab {
                SettingsTab::General => general_panel(
                    palette,
                    app_theme,
                    poll,
                    poll_ms,
                    &section_id,
                    default_section,
                    menubar,
                    menubar_only,
                ),
                SettingsTab::Alerts => alerts_panel(alerts, palette),
                SettingsTab::About => about_panel(palette),
            }),
    )
    .into()
}

fn settings_tab_bar(
    active: SettingsTab,
    palette: Palette,
    mut active_tab: State<SettingsTab>,
) -> Element {
    rect()
        .horizontal()
        .spacing(8.)
        .children(
            [
                (SettingsTab::General, "General"),
                (SettingsTab::Alerts, "Alerts"),
                (SettingsTab::About, "About"),
            ]
            .into_iter()
            .map(|(tab, label)| settings_tab_button(tab, active, label, palette, active_tab))
            .collect::<Vec<_>>(),
        )
        .into()
}

fn settings_tab_button(
    tab: SettingsTab,
    active: SettingsTab,
    tab_label: &'static str,
    palette: Palette,
    mut active_tab: State<SettingsTab>,
) -> Element {
    let is_active = tab == active;
    let bg = if is_active {
        palette.selected_bg()
    } else {
        palette.bg
    };
    let border = if is_active {
        Border::new().fill(palette.accent).width(1.5)
    } else {
        palette.border()
    };

    rect()
        .padding(Gaps::new(8., 14., 8., 14.))
        .background(bg)
        .corner_radius(8.)
        .border(border)
        .on_mouse_up(move |_| active_tab.set(tab))
        .child(
            label()
                .text(tab_label)
                .font_size(12.)
                .font_weight(if is_active {
                    FontWeight::BOLD
                } else {
                    FontWeight::NORMAL
                })
                .color(if is_active {
                    palette.text
                } else {
                    palette.muted
                }),
        )
        .into()
}

fn panel_shell(palette: Palette, body: impl IntoElement) -> Element {
    rect()
        .vertical()
        .width(Size::fill())
        .background(palette.panel)
        .corner_radius(12.)
        .border(palette.border())
        .padding(Gaps::new_all(12.))
        .spacing(14.)
        .child(body)
        .into()
}

fn general_panel(
    palette: Palette,
    app_theme: State<AppTheme>,
    poll: u64,
    poll_ms: State<u64>,
    section_id: &str,
    default_section: State<String>,
    menubar: bool,
    menubar_only: State<bool>,
) -> Element {
    panel_shell(
        palette,
        rect()
            .vertical()
            .spacing(14.)
            .child(settings_section_heading("Sampling", palette))
            .child(settings_hint(
                "How often network stats refresh. Takes effect immediately.",
                palette,
            ))
            .child(poll_interval_picker(poll, palette, poll_ms))
            .child(settings_section_heading("Startup", palette))
            .child(settings_hint(
                "Sidebar section shown the next time you launch Osman.",
                palette,
            ))
            .child(default_section_picker(section_id, palette, default_section))
            .child(settings_section_heading("Appearance", palette))
            .child(theme_picker_section(palette, app_theme))
            .child(settings_section_heading("Menubar", palette))
            .child(menubar_only_toggle(menubar, palette, menubar_only))
            .child(settings_section_heading("Platform", palette))
            .child(settings_hint(
                "Process and connection views require macOS nettop/lsof.",
                palette,
            )),
    )
}

fn alerts_panel(alerts: &AlertEngine, palette: Palette) -> Element {
    rect()
        .vertical()
        .width(Size::fill())
        .spacing(8.)
        .child(settings_section_heading("Alert rules", palette))
        .child(settings_hint(
            "Same rules as the Alerts sidebar screen. Editing thresholds ships in Phase 1B.",
            palette,
        ))
        .child(alerts_screen(alerts, palette))
        .into()
}

fn about_panel(palette: Palette) -> Element {
    panel_shell(
        palette,
        rect()
            .vertical()
            .spacing(12.)
            .child(settings_section_heading("Osman", palette))
            .child(
                label()
                    .text(format!("Version {APP_VERSION}"))
                    .font_size(13.)
                    .color(palette.text),
            )
            .child(settings_hint(
                "Native Mac network monitor — adapters, traffic character, processes, and alerts.",
                palette,
            ))
            .child(
                Button::new()
                    .on_press(|_| menubar::request_about_window())
                    .padding(Gaps::new(10., 16., 10., 16.))
                    .background(palette.accent)
                    .corner_radius(8.)
                    .child(
                        label()
                            .text("About Osman…")
                            .font_size(13.)
                            .font_weight(FontWeight::BOLD)
                            .color(Color::from_rgb(255, 255, 255)),
                    ),
            ),
    )
}

fn poll_interval_picker(active_ms: u64, palette: Palette, poll_ms: State<u64>) -> Element {
    rect()
        .horizontal()
        .spacing(8.)
        .children(
            POLL_OPTIONS
                .iter()
                .map(|(ms, label)| poll_option(*ms, active_ms, label, palette, poll_ms))
                .collect::<Vec<_>>(),
        )
        .into()
}

fn poll_option(
    ms: u64,
    active_ms: u64,
    option_label: &'static str,
    palette: Palette,
    mut poll_ms: State<u64>,
) -> Element {
    let is_active = ms == active_ms;
    let bg = if is_active {
        palette.selected_bg()
    } else {
        palette.bg
    };
    let border = if is_active {
        Border::new().fill(palette.accent).width(1.5)
    } else {
        palette.border()
    };

    rect()
        .padding(Gaps::new(8., 14., 8., 14.))
        .background(bg)
        .corner_radius(8.)
        .border(border)
        .on_mouse_up(move |_| {
            let _ = preferences::with_store(|store| store.set_poll_interval_ms(ms));
            poll_ms.set(ms);
        })
        .child(
            label()
                .text(option_label)
                .font_size(12.)
                .font_weight(if is_active {
                    FontWeight::BOLD
                } else {
                    FontWeight::NORMAL
                })
                .color(if is_active {
                    palette.text
                } else {
                    palette.muted
                }),
        )
        .into()
}

fn default_section_picker(
    active_id: &str,
    palette: Palette,
    default_section: State<String>,
) -> Element {
    let active = active_id.to_string();
    rect()
        .vertical()
        .spacing(6.)
        .children(
            DEFAULT_SECTIONS
                .iter()
                .map(|(id, label)| section_option(id, label, &active, palette, default_section))
                .collect::<Vec<_>>(),
        )
        .into()
}

fn section_option(
    id: &str,
    section_label: &str,
    active_id: &str,
    palette: Palette,
    mut default_section: State<String>,
) -> Element {
    let is_active = id == active_id;
    let bg = if is_active {
        palette.selected_bg()
    } else {
        palette.bg
    };
    let border = if is_active {
        Border::new().fill(palette.accent).width(1.5)
    } else {
        palette.border()
    };
    let id_owned = id.to_string();
    let label_owned = section_label.to_string();

    rect()
        .horizontal()
        .width(Size::fill())
        .padding(Gaps::new(8., 10., 8., 10.))
        .background(bg)
        .corner_radius(8.)
        .border(border)
        .on_mouse_up(move |_| {
            let _ = preferences::with_store(|store| store.set_default_section(&id_owned));
            default_section.set(id_owned.clone());
        })
        .child(
            label()
                .text(label_owned)
                .font_size(12.)
                .font_weight(if is_active {
                    FontWeight::BOLD
                } else {
                    FontWeight::NORMAL
                })
                .color(if is_active {
                    palette.text
                } else {
                    palette.muted
                }),
        )
        .child(
            label()
                .text(if is_active { "Default" } else { "" })
                .font_size(10.)
                .color(palette.muted),
        )
        .into()
}

fn menubar_only_toggle(
    enabled: bool,
    palette: Palette,
    mut menubar_only: State<bool>,
) -> Element {
    let status = if enabled { "On" } else { "Off" };
    let status_color = if enabled {
        palette.receive
    } else {
        palette.muted
    };
    let next = !enabled;

    rect()
        .horizontal()
        .width(Size::fill())
        .padding(Gaps::new(10., 12., 10., 12.))
        .background(palette.bg)
        .corner_radius(8.)
        .border(palette.border())
        .on_mouse_up(move |_| {
            let _ = preferences::with_store(|store| store.set_menubar_only(next));
            macos_activation::set_menubar_only(next);
            menubar_only.set(next);
        })
        .child(
            rect()
                .vertical()
                .spacing(2.)
                .child(
                    label()
                        .text("Menubar only")
                        .font_size(12.)
                        .font_weight(FontWeight::BOLD)
                        .color(palette.text),
                )
                .child(
                    label()
                        .text("Hide the Dock icon — open Osman from the menu bar.")
                        .font_size(10.)
                        .color(palette.muted),
                ),
        )
        .child(
            label()
                .text(status)
                .font_size(11.)
                .font_weight(FontWeight::BOLD)
                .color(status_color),
        )
        .into()
}

fn theme_picker_section(palette: Palette, app_theme: State<AppTheme>) -> Element {
    let active = *app_theme.read();
    rect()
        .vertical()
        .spacing(8.)
        .child(settings_hint(
            "Receive, send, and total waveform colors plus surface tint.",
            palette,
        ))
        .child(
            rect()
                .vertical()
                .spacing(6.)
                .children(
                    AppTheme::ALL
                        .iter()
                        .map(|theme| theme_option(*theme, active, palette, app_theme))
                        .collect::<Vec<_>>(),
                ),
        )
        .into()
}

fn theme_option(
    theme: AppTheme,
    active: AppTheme,
    palette: Palette,
    mut app_theme: State<AppTheme>,
) -> Element {
    let preview = theme.palette();
    let is_active = theme == active;
    let bg = if is_active {
        Color::from_argb(40, preview.receive.r(), preview.receive.g(), preview.receive.b())
    } else {
        palette.bg
    };
    let border = if is_active {
        Border::new().fill(preview.accent).width(1.5)
    } else {
        palette.border()
    };

    rect()
        .horizontal()
        .width(Size::fill())
        .padding(Gaps::new(8., 10., 8., 10.))
        .background(bg)
        .corner_radius(8.)
        .border(border)
        .spacing(10.)
        .on_mouse_up(move |_| {
            let _ = preferences::with_store(|store| store.set_theme(theme));
            app_theme.set(theme);
        })
        .child(theme_swatches(preview))
        .child(
            rect()
                .vertical()
                .spacing(2.)
                .child(
                    label()
                        .text(theme.label())
                        .font_size(12.)
                        .font_weight(if is_active {
                            FontWeight::BOLD
                        } else {
                            FontWeight::NORMAL
                        })
                        .color(if is_active {
                            palette.text
                        } else {
                            palette.muted
                        }),
                )
                .child(
                    label()
                        .text(if is_active {
                            "Active"
                        } else {
                            "Click to apply"
                        })
                        .font_size(10.)
                        .color(palette.muted),
                ),
        )
        .into()
}

fn theme_swatches(preview: Palette) -> Element {
    rect()
        .horizontal()
        .spacing(4.)
        .children(
            [preview.receive, preview.send, preview.total]
                .into_iter()
                .map(|color| {
                    rect()
                        .width(Size::px(14.))
                        .height(Size::px(14.))
                        .corner_radius(7.)
                        .background(color)
                        .into()
                })
                .collect::<Vec<_>>(),
        )
        .into()
}

fn settings_section_heading(title: &str, palette: Palette) -> Element {
    label()
        .text(title.to_string())
        .font_size(14.)
        .font_weight(FontWeight::BOLD)
        .color(palette.title)
        .into()
}

fn settings_hint(text: &str, palette: Palette) -> Element {
    label()
        .text(text.to_string())
        .font_size(11.)
        .color(palette.muted)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poll_options_match_preferences_normalizer() {
        for (ms, _) in POLL_OPTIONS {
            assert_eq!(preferences::normalize_poll_ms(*ms), *ms);
        }
    }

    #[test]
    fn default_sections_use_known_ids() {
        assert!(DEFAULT_SECTIONS.iter().any(|(id, _)| *id == "overview"));
        assert!(DEFAULT_SECTIONS.iter().any(|(id, _)| *id == "settings"));
    }
}

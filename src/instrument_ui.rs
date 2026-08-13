//! Shared instrument-cluster UI — sidebar chrome, badges, narrative, chips.

use freya::components::CanvasContext;
use freya::engine::prelude::{FilterMode, MipmapMode, Paint, Rect as SkRect, SamplingOptions};
use freya::prelude::*;

use crate::adapters::adapter_title;
use crate::detail::{ConnectionDetail, ProcessTraffic};
use crate::icon_assets::SIDEBAR_SCOPE;
use crate::network::NetworkSnapshot;
use crate::rate_tracker::LiveConnectionRate;
use crate::theme::{format_rate, Palette, ProcessLane};
use crate::time_window::TimeWindow;
use crate::traffic_character::{
    classify_interface, personality_from_character, top_talker_for_interface, AdapterPersonality,
    TrafficCharacter,
};

pub fn sidebar_scope_mark() -> Element {
    rect()
        .width(Size::px(28.))
        .height(Size::px(28.))
        .corner_radius(6.)
        .overflow(Overflow::Clip)
        .child(
            canvas(RenderCallback::new(|ctx| draw_scope_mark(ctx)))
                .width(Size::fill())
                .height(Size::fill()),
        )
        .into()
}

fn draw_scope_mark(ctx: &mut CanvasContext) {
    let w = ctx.size.width.max(1.0);
    let h = ctx.size.height.max(1.0);
    let img = &SIDEBAR_SCOPE.image;
    let iw = img.width().max(1) as f32;
    let ih = img.height().max(1) as f32;
    let scale = (w / iw).min(h / ih);
    let dw = iw * scale;
    let dh = ih * scale;
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

pub fn sidebar_compact_rates(snapshot: &NetworkSnapshot, palette: Palette) -> Element {
    let total = snapshot.total_rx_bps + snapshot.total_tx_bps;
    rect()
        .vertical()
        .spacing(4.)
        .child(sidebar_rate_line('↓', snapshot.total_rx_bps, palette.receive, palette))
        .child(sidebar_rate_line('↑', snapshot.total_tx_bps, palette.send, palette))
        .child(sidebar_rate_line('Σ', total, palette.total, palette))
        .into()
}

fn sidebar_rate_line(
    prefix: char,
    rate: f64,
    color: Color,
    palette: Palette,
) -> Element {
    label()
        .text(format!("{prefix} {}", format_rate(rate)))
        .font_size(11.)
        .font_weight(FontWeight::BOLD)
        .color(color)
        .into()
}

pub fn nav_group_heading(title: &'static str, palette: Palette) -> Element {
    label()
        .text(title)
        .font_size(9.)
        .font_weight(FontWeight::BOLD)
        .color(palette.muted)
        .into()
}

pub fn personality_badge(personality: AdapterPersonality, palette: Palette) -> Element {
    let (bg, fg) = match personality {
        AdapterPersonality::Steady => (
            Color::from_argb(36, palette.receive.r(), palette.receive.g(), palette.receive.b()),
            palette.receive,
        ),
        AdapterPersonality::Bursty => (
            Color::from_argb(36, palette.send.r(), palette.send.g(), palette.send.b()),
            palette.send,
        ),
        AdapterPersonality::Idle => (
            Color::from_argb(24, palette.muted.r(), palette.muted.g(), palette.muted.b()),
            palette.muted,
        ),
    };
    rect()
        .padding(Gaps::new(3., 8., 3., 8.))
        .background(bg)
        .corner_radius(10.)
        .child(
            label()
                .text(personality.label())
                .font_size(9.)
                .font_weight(FontWeight::BOLD)
                .color(fg),
        )
        .into()
}

pub fn process_letter_mark(name: &str, palette: Palette) -> Element {
    let letter = name
        .chars()
        .next()
        .unwrap_or('?')
        .to_uppercase()
        .collect::<String>();
    rect()
        .width(Size::px(20.))
        .height(Size::px(20.))
        .main_align(Alignment::Center)
        .cross_align(Alignment::Center)
        .background(Color::from_argb(28, palette.receive.r(), palette.receive.g(), palette.receive.b()))
        .corner_radius(6.)
        .child(
            label()
                .text(letter)
                .font_size(10.)
                .font_weight(FontWeight::BOLD)
                .color(palette.text),
        )
        .into()
}

pub fn primary_adapter_label(snapshot: &NetworkSnapshot) -> String {
    snapshot
        .interfaces
        .iter()
        .find(|i| i.name == "en0")
        .or_else(|| {
            snapshot.interfaces.iter().max_by(|a, b| {
                a.combined_bps
                    .partial_cmp(&b.combined_bps)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        })
        .map(|i| adapter_title(&i.name))
        .unwrap_or_else(|| "All adapters".into())
}

pub fn primary_adapter_short(snapshot: &NetworkSnapshot) -> String {
    snapshot
        .interfaces
        .iter()
        .find(|i| i.name == "en0")
        .or_else(|| snapshot.interfaces.first())
        .map(|i| adapter_title(&i.name))
        .unwrap_or_else(|| "Wi-Fi".into())
}

pub fn overview_narrative_line(
    snapshot: &NetworkSnapshot,
    connections: &[ConnectionDetail],
    processes: &[ProcessTraffic],
    live_rates: &[LiveConnectionRate],
    window: TimeWindow,
) -> String {
    let adapter = primary_adapter_short(snapshot);
    let iface = snapshot
        .interfaces
        .iter()
        .find(|i| i.name == "en0")
        .or_else(|| snapshot.interfaces.first());

    let Some(iface) = iface else {
        return format!("Waiting for adapters · last {}", window.label());
    };

    let iface_conns: Vec<ConnectionDetail> = connections
        .iter()
        .filter(|c| c.interface == iface.name)
        .cloned()
        .collect();
    let (character, _) = classify_interface(iface, &iface_conns);
    let personality = personality_from_character(character);

    if iface.combined_bps < 800.0 {
        return format!("Quiet on {adapter} · last {}", window.label());
    }

    let talker_name = top_talker_for_interface(&iface_conns, processes);
    let talker_live = live_rates
        .iter()
        .filter(|r| r.interface == iface.name)
        .map(|r| r.combined_bps())
        .fold(0.0_f64, f64::max);
    let talker_rate = if talker_live > 0.0 {
        format_rate(talker_live)
    } else {
        format_rate(iface.combined_bps)
    };

    let spike = iface.tx_bps > iface.rx_bps * 1.2 && iface.tx_bps > 50_000.0;
    let active = iface_conns.len().max(1);

    match personality {
        AdapterPersonality::Bursty if spike => {
            format!("↑ send spike on {adapter} · {talker_name} {talker_rate} · {active} active sockets")
        }
        AdapterPersonality::Idle => format!("Quiet on {adapter} · last {}", window.label()),
        _ => format!(
            "Live on {adapter} · {talker_name} {talker_rate} · {active} active sockets"
        ),
    }
}

pub fn time_window_chips(
    time_window: State<TimeWindow>,
    active: TimeWindow,
    palette: Palette,
) -> Element {
    rect()
        .horizontal()
        .spacing(6.)
        .children(
            [TimeWindow::Sec60, TimeWindow::Min5, TimeWindow::Min15]
                .into_iter()
                .map(|window| {
                    time_window_chip(time_window, active, window, palette)
                })
                .collect::<Vec<_>>(),
        )
        .into()
}

fn time_window_chip(
    mut time_window: State<TimeWindow>,
    active: TimeWindow,
    window: TimeWindow,
    palette: Palette,
) -> Element {
    let selected = active == window;
    let border = if selected {
        palette.text
    } else {
        palette.panel_edge
    };
    rect()
        .padding(Gaps::new(4., 10., 4., 10.))
        .border(
            Border::new()
                .fill(border)
                .width(BorderWidth {
                    top: 1.,
                    right: 1.,
                    bottom: 1.,
                    left: 1.,
                }),
        )
        .corner_radius(6.)
        .on_mouse_up(move |e: Event<MouseEventData>| {
            e.stop_propagation();
            *time_window.write_unchecked() = window;
        })
        .child(
            label()
                .text(window.label())
                .font_size(10.)
                .font_weight(if selected {
                    FontWeight::BOLD
                } else {
                    FontWeight::NORMAL
                })
                .color(if selected {
                    palette.text
                } else {
                    palette.muted
                }),
        )
        .into()
}

pub fn alerts_chip(
    count: usize,
    on_press: impl FnMut(Event<MouseEventData>) + 'static,
    palette: Palette,
) -> Element {
    if count == 0 {
        return rect().width(Size::px(0.)).height(Size::px(0.)).into();
    }
    rect()
        .padding(Gaps::new(4., 10., 4., 10.))
        .background(Color::from_argb(28, palette.send.r(), palette.send.g(), palette.send.b()))
        .corner_radius(10.)
        .border(
            Border::new()
                .fill(palette.send)
                .width(BorderWidth {
                    top: 1.,
                    right: 1.,
                    bottom: 1.,
                    left: 1.,
                }),
        )
        .on_mouse_up(on_press)
        .child(
            label()
                .text(format!("{count} alerts"))
                .font_size(10.)
                .font_weight(FontWeight::BOLD)
                .color(palette.send),
        )
        .into()
}

pub fn inspect_mode_toggle(
    processes_active: bool,
    on_processes: impl FnMut(Event<MouseEventData>) + 'static,
    on_connections: impl FnMut(Event<MouseEventData>) + 'static,
    palette: Palette,
) -> Element {
    rect()
        .horizontal()
        .spacing(16.)
        .child(inspect_toggle_tab(
            "Processes",
            processes_active,
            on_processes,
            palette,
        ))
        .child(inspect_toggle_tab(
            "Connections",
            !processes_active,
            on_connections,
            palette,
        ))
        .into()
}

fn inspect_toggle_tab(
    label_text: &'static str,
    selected: bool,
    on_press: impl FnMut(Event<MouseEventData>) + 'static,
    palette: Palette,
) -> Element {
    rect()
        .padding(Gaps::new(0., 0., 6., 0.))
        .border(if selected {
            Border::new()
                .fill(palette.receive)
                .width(BorderWidth {
                    top: 0.,
                    right: 0.,
                    bottom: 2.,
                    left: 0.,
                })
        } else {
            Border::new().width(BorderWidth::default())
        })
        .on_mouse_up(on_press)
        .child(
            label()
                .text(label_text)
                .font_size(11.)
                .font_weight(if selected {
                    FontWeight::BOLD
                } else {
                    FontWeight::NORMAL
                })
                .color(if selected {
                    palette.text
                } else {
                    palette.muted
                }),
        )
        .into()
}

pub fn activity_header_label(window: TimeWindow) -> &'static str {
    match window {
        TimeWindow::Sec60 => "Activity (last 60s)",
        TimeWindow::Min5 => "Activity (last 5m)",
        TimeWindow::Min15 => "Activity (last 15m)",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traffic_character::{personality_from_character, TrafficCharacter};

    #[test]
    fn personality_maps_steady_stream() {
        assert_eq!(
            personality_from_character(TrafficCharacter::SteadyStream),
            AdapterPersonality::Steady
        );
    }

    #[test]
    fn personality_maps_chaotic_to_bursty() {
        assert_eq!(
            personality_from_character(TrafficCharacter::ChaoticMultiplex),
            AdapterPersonality::Bursty
        );
    }

    #[test]
    fn personality_maps_idle() {
        assert_eq!(
            personality_from_character(TrafficCharacter::ListenIdle),
            AdapterPersonality::Idle
        );
    }
}

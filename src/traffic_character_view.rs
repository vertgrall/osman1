use freya::prelude::*;

use crate::adapters::{adapter_title, scope_id};
use crate::character_render::{CharacterDrawProfile, CharacterScopeBank};
use crate::character_timeline::{draw_character_timeline, CharacterTimeline};
use crate::detail::{ConnectionDetail, ProcessTraffic};
use crate::network::{InterfaceStats, NetworkSnapshot};
use crate::rate_tracker::{rates_for_interface, LiveConnectionRate};
use crate::theme::{format_rate, Palette, ProcessLane};
use crate::time_window::TimeWindow;
use crate::traffic_character::{
    behavior_note, classify_interface, connections_for_interface, top_talker_for_interface,
    ProtocolKind, TrafficCharacter,
};

pub fn traffic_character_screen(
    snapshot: NetworkSnapshot,
    connections: Vec<ConnectionDetail>,
    processes: Vec<ProcessTraffic>,
    live_rates: Vec<LiveConnectionRate>,
    palette: Palette,
    anim_clock: State<f64>,
    character_scopes: State<CharacterScopeBank>,
    timeline: CharacterTimeline,
    window: TimeWindow,
) -> Element {
    let mut interfaces = snapshot.interfaces.clone();
    interfaces.sort_by(|a, b| {
        b.combined_bps
            .partial_cmp(&a.combined_bps)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let rows: Vec<Element> = interfaces
        .iter()
        .enumerate()
        .map(|(index, iface)| {
            character_adapter_row(
                iface,
                index,
                &connections,
                &processes,
                &live_rates,
                palette,
                anim_clock,
                character_scopes,
                &timeline,
                window,
            )
        })
        .collect();

    rect()
        .vertical()
        .expanded()
        .padding(Gaps::new_all(16.))
        .spacing(12.)
        .child(
            label()
                .text("Traffic Character")
                .font_size(20.)
                .font_weight(FontWeight::BOLD)
                .color(palette.title),
        )
        .child(
            ScrollView::new()
                .expanded()
                .spacing(12.)
                .child(
                    label()
                        .text("Animation legend — demo waveforms (not live traffic)")
                        .font_size(12.)
                        .color(palette.muted),
                )
                .child(character_legend_grid(palette, anim_clock, character_scopes))
                .child(
                    label()
                        .text("Live adapters")
                        .font_size(14.)
                        .font_weight(FontWeight::BOLD)
                        .color(palette.title),
                )
                .child(character_adapter_table(palette, rows))
                .child(traffic_character_footer(palette)),
        )
        .into()
}

fn character_legend_grid(
    palette: Palette,
    anim_clock: State<f64>,
    character_scopes: State<CharacterScopeBank>,
) -> Element {
    let cards: Vec<Element> = TrafficCharacter::all()
        .into_iter()
        .map(|character| character_legend_card(character, palette, anim_clock, character_scopes))
        .collect();

    let mut rows = Vec::new();
    for chunk in cards.chunks(3) {
        rows.push(
            rect()
                .horizontal()
                .width(Size::fill())
                .spacing(10.)
                .children(chunk.to_vec())
                .into(),
        );
    }

    rect()
        .vertical()
        .width(Size::fill())
        .spacing(10.)
        .children(rows)
        .into()
}

fn character_legend_card(
    character: TrafficCharacter,
    palette: Palette,
    anim_clock: State<f64>,
    character_scopes: State<CharacterScopeBank>,
) -> Element {
    let frame_base = (*anim_clock.peek() * 60.0) as u64;
    let scope_key = character.scope_id();
    let card_color = character.primary_color(palette);

    rect()
        .vertical()
        .width(Size::percent(33.))
        .spacing(6.)
        .padding(Gaps::new_all(10.))
        .background(palette.panel)
        .corner_radius(12.)
        .border(palette.border())
        .child(
            rect()
                .horizontal()
                .spacing(6.)
                .width(Size::fill())
                .child(
                    rect()
                        .horizontal()
                        .spacing(6.)
                        .child(
                            rect()
                                .width(Size::px(8.))
                                .height(Size::px(8.))
                                .background(card_color)
                                .corner_radius(4.),
                        )
                        .child(
                            label()
                                .text(character.title())
                                .font_size(12.)
                                .font_weight(FontWeight::BOLD)
                                .color(palette.text),
                        ),
                )
                .child(
                    rect()
                        .padding(Gaps::new(2., 6., 2., 6.))
                        .background(palette.bg)
                        .corner_radius(4.)
                        .border(palette.border())
                        .child(
                            label()
                                .text("Demo")
                                .font_size(9.)
                                .font_weight(FontWeight::BOLD)
                                .color(palette.muted),
                        ),
                ),
        )
        .child(
            rect()
                .width(Size::fill())
                .height(Size::px(3.))
                .background(card_color)
                .corner_radius(2.),
        )
        .child(
            label()
                .text(format!("Sample · {}", format_rate(character.demo_bps())))
                .font_size(9.)
                .color(palette.muted),
        )
        .child(
            canvas(RenderCallback::new(move |ctx| {
                let t = *anim_clock.peek();
                character_scopes
                    .write_unchecked()
                    .draw_demo(ctx, character, t, palette);
            }))
            .width(Size::fill())
            .height(Size::px(72.))
            .key(scope_key.wrapping_add(frame_base)),
        )
        .child(
            label()
                .text(character.detection_hint())
                .font_size(10.)
                .color(palette.muted),
        )
        .into()
}

fn character_adapter_table(palette: Palette, rows: Vec<Element>) -> Element {
    rect()
        .vertical()
        .width(Size::fill())
        .spacing(6.)
        .background(palette.panel)
            .corner_radius(12.)
            .border(palette.elevated_border())
        .padding(Gaps::new_all(12.))
        .child(character_table_header(palette))
        .children(if rows.is_empty() {
            vec![empty_adapter_state(palette)]
        } else {
            rows
        })
        .into()
}

fn empty_adapter_state(palette: Palette) -> Element {
    rect()
        .padding(Gaps::new_all(20.))
        .child(
            label()
                .text("No adapters sampled yet.")
                .font_size(13.)
                .color(palette.muted),
        )
        .into()
}

fn character_table_header(palette: Palette) -> Element {
    list_header(
        palette,
        &[
            ("Adapter", 22.),
            ("Character Example (Last 60s)", 28.),
            ("Detected Pattern", 16.),
            ("Behavior Notes", 20.),
            ("Top Talker", 14.),
        ],
    )
}

fn character_adapter_row(
    iface: &InterfaceStats,
    index: usize,
    connections: &[ConnectionDetail],
    processes: &[ProcessTraffic],
    live_rates: &[LiveConnectionRate],
    palette: Palette,
    anim_clock: State<f64>,
    character_scopes: State<CharacterScopeBank>,
    timeline: &CharacterTimeline,
    window: TimeWindow,
) -> Element {
    let iface_connections: Vec<ConnectionDetail> = connections_for_interface(&iface.name, connections)
        .into_iter()
        .cloned()
        .collect();
    let (character, _protocol) = classify_interface(iface, &iface_connections);
    let trace_key = scope_id(&iface.name, ProcessLane::Green);
    let note = behavior_note(character, &iface.name);
    let iface_rates = rates_for_interface(live_rates, &iface.name);
    let talker = top_talker_live(&iface_rates, processes, &iface_connections);
    let talker_pct = talker_share_pct_live(&talker, &iface_rates);
    let title = adapter_title(&iface.name);
    let combined_bps = iface.combined_bps;
    let time = *anim_clock.peek();
    let frame_key = (time * 60.0) as u64;
    let segments = timeline.segments_for(&iface.name, window);
    let sample_index = timeline.sample_index();
    let talker_label = format_talker_label(&talker.0);

    rect()
        .horizontal()
        .width(Size::fill())
        .height(Size::px(78.))
        .padding(Gaps::new_all(10.))
        .background(palette.row_bg(index, false))
        .corner_radius(8.)
        .border(palette.border())
        .spacing(8.)
        .child(
            rect()
                .vertical()
                .spacing(3.)
                .width(Size::percent(22.))
                .child(
                    label()
                        .text(title)
                        .font_size(12.)
                        .font_weight(FontWeight::BOLD)
                        .color(palette.text),
                )
                .child(
                    label()
                        .text(format_rate(iface.combined_bps))
                        .font_size(10.)
                        .color(palette.muted),
                ),
        )
        .child(
            rect()
                .vertical()
                .spacing(4.)
                .width(Size::percent(28.))
                .child(
                    canvas(RenderCallback::new(move |ctx| {
                        character_scopes.write_unchecked().draw_live(
                            ctx,
                            trace_key,
                            character,
                            time,
                            palette,
                            CharacterDrawProfile::ADAPTER_ROW,
                            Some(combined_bps),
                        );
                    }))
                    .width(Size::fill())
                    .height(Size::px(52.))
                    .key(trace_key.wrapping_add(frame_key)),
                )
                .child(
                    canvas(RenderCallback::new(move |ctx| {
                        draw_character_timeline(
                            ctx,
                            &segments,
                            sample_index,
                            window,
                            palette,
                        );
                    }))
                    .width(Size::fill())
                    .height(Size::px(10.)),
                ),
        )
        .child(
            label()
                .text(character.title())
                .font_size(11.)
                .font_weight(FontWeight::BOLD)
                .color(character.primary_color(palette))
                .width(Size::percent(16.)),
        )
        .child(
            label()
                .text(note)
                .font_size(10.)
                .color(palette.muted)
                .width(Size::percent(20.)),
        )
        .child(
            rect()
                .vertical()
                .spacing(4.)
                .width(Size::percent(14.))
                .child(
                    label()
                        .text(talker_label)
                        .font_size(10.)
                        .font_weight(FontWeight::BOLD)
                        .color(palette.text),
                )
                .child(
                    rect()
                        .width(Size::fill())
                        .height(Size::px(5.))
                        .background(palette.panel)
                        .corner_radius(3.)
                        .child(
                            rect()
                                .width(Size::percent(talker_pct))
                                .height(Size::px(5.))
                                .background(palette.total)
                                .corner_radius(3.),
                        ),
                )
                .child(
                    label()
                        .text(if talker.1 > 0.0 {
                            format_rate(talker.1)
                        } else {
                            "—".into()
                        })
                        .font_size(10.)
                        .font_weight(FontWeight::BOLD)
                        .color(palette.text),
                ),
        )
        .into()
}

fn format_talker_label(name: &str) -> String {
    if name == "—" {
        "—".into()
    } else if name.contains(':') || name.starts_with("listen") {
        format!("Remote: {name}")
    } else {
        format!("Process: {name}")
    }
}

fn top_talker_live(
    live_rates: &[LiveConnectionRate],
    processes: &[ProcessTraffic],
    connections: &[ConnectionDetail],
) -> (String, f64) {
    if let Some(top) = live_rates.first() {
        let name = if top.remote_label.is_empty() || top.remote_label == "—" {
            top.process_name.clone()
        } else {
            top.remote_label.clone()
        };
        return (name, top.combined_bps());
    }
    let name = top_talker_for_interface(connections, processes);
    (name, 0.0)
}

fn talker_share_pct_live(talker: &(String, f64), live_rates: &[LiveConnectionRate]) -> f32 {
    if talker.1 <= 0.0 {
        return 8.0;
    }
    let max = live_rates
        .iter()
        .map(|r| r.combined_bps())
        .fold(1.0_f64, f64::max)
        .max(1.0);
    ((talker.1 / max) * 100.0).clamp(8.0, 100.0) as f32
}

pub fn traffic_character_footer(palette: Palette) -> Element {
    rect()
        .vertical()
        .width(Size::fill())
        .spacing(8.)
        .padding(Gaps::new_all(12.))
        .background(palette.panel)
        .corner_radius(12.)
        .border(palette.border())
        .child(
            rect()
                .horizontal()
                .spacing(12.)
                .child(footer_pill("Color = Direction", palette))
                .child(footer_lane_chip(ProcessLane::Red, "Receive", palette))
                .child(footer_lane_chip(ProcessLane::Blue, "Send", palette))
                .child(footer_lane_chip(ProcessLane::Green, "Total", palette))
                .child(footer_pill("Motion = Behavior", palette))
                .child(footer_pill("Smooth = Stream", palette))
                .child(footer_pill("Saw = Batch", palette))
                .child(footer_pill("Pulse = API", palette)),
        )
        .child(
            rect()
                .horizontal()
                .spacing(12.)
                .child(footer_pill("Label = Protocol", palette))
                .child(protocol_legend_item(ProtocolKind::Tcp, palette))
                .child(protocol_legend_item(ProtocolKind::Udp, palette))
                .child(protocol_legend_item(ProtocolKind::Icmp, palette)),
        )
        .into()
}

fn footer_pill(text: &'static str, palette: Palette) -> Element {
    label()
        .text(text)
        .font_size(10.)
        .font_weight(FontWeight::BOLD)
        .color(palette.muted)
        .into()
}

fn footer_lane_chip(lane: ProcessLane, text: &'static str, palette: Palette) -> Element {
    rect()
        .horizontal()
        .spacing(5.)
        .child(
            rect()
                .width(Size::px(7.))
                .height(Size::px(7.))
                .background(lane.color(palette))
                .corner_radius(4.),
        )
        .child(
            label()
                .text(text)
                .font_size(10.)
                .color(palette.text),
        )
        .into()
}

fn protocol_legend_item(protocol: ProtocolKind, palette: Palette) -> Element {
    rect()
        .horizontal()
        .spacing(6.)
        .child(
            canvas(RenderCallback::new(move |ctx| {
                let width = ctx.size.width.max(1.0);
                let height = ctx.size.height.max(1.0);
                let y = height * 0.5;
                let color = palette.total;
                let mut paint = freya::engine::prelude::Paint::default();
                paint.set_anti_alias(true);
                paint.set_style(freya::engine::prelude::PaintStyle::Stroke);
                paint.set_stroke_width(1.5);
                paint.set_color4f(
                    freya::engine::prelude::Color4f::new(
                        color.r() as f32 / 255.0,
                        color.g() as f32 / 255.0,
                        color.b() as f32 / 255.0,
                        0.95,
                    ),
                    None,
                );
                if protocol.dashed_sparkline() {
                    draw_dashed_horizontal_line(ctx, width, y, &paint);
                } else {
                    ctx.canvas.draw_line(
                        freya::engine::prelude::Point::new(0.0, y),
                        freya::engine::prelude::Point::new(width, y),
                        &paint,
                    );
                }
                if matches!(protocol, ProtocolKind::Icmp) {
                    let cx = width * 0.5;
                    let s = 2.5;
                    let mut path = freya::engine::prelude::PathBuilder::new();
                    path.move_to(freya::engine::prelude::Point::new(cx, y - s));
                    path.line_to(freya::engine::prelude::Point::new(cx + s, y));
                    path.line_to(freya::engine::prelude::Point::new(cx, y + s));
                    path.line_to(freya::engine::prelude::Point::new(cx - s, y));
                    path.close();
                    paint.set_style(freya::engine::prelude::PaintStyle::Fill);
                    ctx.canvas.draw_path(&path.detach(), &paint);
                }
            }))
            .width(Size::px(28.))
            .height(Size::px(10.)),
        )
        .child(
            label()
                .text(format!("{} ({})", protocol.label(), protocol.detail()))
                .font_size(10.)
                .color(palette.text),
        )
        .into()
}

fn draw_dashed_horizontal_line(
    ctx: &freya::components::CanvasContext,
    width: f32,
    y: f32,
    paint: &freya::engine::prelude::Paint,
) {
    let mut x = 0.0;
    while x < width {
        let end = (x + 8.0).min(width);
        ctx.canvas.draw_line(
            freya::engine::prelude::Point::new(x, y),
            freya::engine::prelude::Point::new(end, y),
            paint,
        );
        x += 11.0;
    }
}

fn list_header(palette: Palette, columns: &[(&str, f32)]) -> Element {
    rect()
        .horizontal()
        .width(Size::fill())
        .padding(Gaps::new(0., 10., 6., 10.))
        .spacing(8.)
        .children(
            columns
                .iter()
                .map(|(col_label, width)| {
                    label()
                        .text((*col_label).to_string())
                        .font_size(10.)
                        .font_weight(FontWeight::BOLD)
                        .color(palette.muted)
                        .width(Size::percent(*width))
                        .into()
                })
                .collect::<Vec<_>>(),
        )
        .into()
}

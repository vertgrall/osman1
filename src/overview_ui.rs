//! Overview screen pieces — shared between the app shell and layout tests.

use freya::components::Canvas;
use freya::prelude::*;

use crate::adapter_table_layout::{
    AdapterTableLayout, AdapterTableMode, ACTIVITY_COL_PX, ADAPTER_NAME_COL_PX,
    HERO_CHART_HEIGHT, MIN_RATE_LABEL_WIDTH, MIN_SPARKLINE_WIDTH, OVERVIEW_STATIC_ADAPTER_ROWS,
    SPARKLINE_HEIGHT, STATUS_COL_PX,
};
use crate::adapters::{adapter_hardware_hint, adapter_title, scope_id};
use crate::charts::{
    display_chart_max, draw_activity_sparkline, draw_network_activity, sparkline_scale,
    ChartScaleBank,
};
use crate::data_health::DataHealth;
use crate::detail::{ConnectionDetail, ProcessTraffic};
use crate::instrument_ui::{
    activity_header_label, alerts_chip, overview_narrative_line, primary_adapter_label,
    primary_adapter_short, personality_badge, time_window_chips,
};
use crate::network::{InterfaceStats, NetworkSnapshot};
use crate::rate_tracker::LiveConnectionRate;
use crate::theme::{format_rate, Palette, ProcessLane};
use crate::time_window::{slice_history, TimeWindow};
use crate::traffic_character::{
    classify_interface, connections_for_interface, personality_from_character,
};

pub fn overview_health_banner(message: impl Into<String>, palette: Palette) -> Element {
    let message = message.into();
    rect()
        .width(Size::fill())
        .background(palette.panel)
        .corner_radius(8.)
        .border(palette.border())
        .padding(Gaps::new_all(12.))
        .child(
            label()
                .text(message)
                .font_size(11.)
                .color(palette.muted),
        )
        .into()
}

pub fn overview_instrument_toolbar(
    snapshot: &NetworkSnapshot,
    time_window: State<TimeWindow>,
    window: TimeWindow,
    alert_count: usize,
    on_alerts: impl FnMut(Event<MouseEventData>) + 'static,
    palette: Palette,
) -> Element {
    let adapter = primary_adapter_label(snapshot);
    rect()
        .horizontal()
        .width(Size::fill())
        .cross_align(Alignment::Center)
        .child(
            rect()
                .vertical()
                .spacing(2.)
                .child(
                    label()
                        .text("Network activity")
                        .font_size(18.)
                        .font_weight(FontWeight::BOLD)
                        .color(palette.title),
                )
                .child(
                    label()
                        .text(format!("Last {} on {adapter}", window.label()))
                        .font_size(11.)
                        .color(palette.muted),
                ),
        )
        .child(rect().width(Size::fill()))
        .child(
            rect()
                .horizontal()
                .spacing(8.)
                .cross_align(Alignment::Center)
                .child(time_window_chips(time_window, window, palette))
                .child(alerts_chip(alert_count, on_alerts, palette)),
        )
        .into()
}

pub fn overview_narrative_banner(
    snapshot: &NetworkSnapshot,
    connections: &[ConnectionDetail],
    processes: &[ProcessTraffic],
    live_rates: &[LiveConnectionRate],
    window: TimeWindow,
    palette: Palette,
) -> Element {
    let line = overview_narrative_line(snapshot, connections, processes, live_rates, window);
    label()
        .text(line)
        .font_size(12.)
        .color(palette.muted)
        .into()
}

pub fn overview_adapter_table(
    snapshot: &NetworkSnapshot,
    connections: &[ConnectionDetail],
    palette: Palette,
    selected: State<Option<String>>,
    window: TimeWindow,
    sample_tick: u64,
    mode: AdapterTableMode,
) -> Element {
    let fill_height = matches!(mode, AdapterTableMode::FullList);

    let mut table = rect()
        .vertical()
        .width(Size::fill())
        .background(palette.panel)
        .corner_radius(12.)
        .border(palette.border())
        .padding(Gaps::new_all(12.))
        .spacing(0.);

    if fill_height {
        table = table.height(Size::fill());
    }

    table
        .child(overview_adapter_header(palette, window))
        .child(overview_adapter_rows(
            snapshot,
            connections,
            palette,
            selected,
            window,
            sample_tick,
            mode,
        ))
        .into()
}

fn overview_adapter_header(palette: Palette, window: TimeWindow) -> Element {
    rect()
        .horizontal()
        .width(Size::fill())
        .padding(Gaps::new(0., 0., 8., 0.))
        .spacing(8.)
        .child(header_label_with_min("Adapter", palette, ADAPTER_NAME_COL_PX))
        .child(header_label_with_min("Status", palette, STATUS_COL_PX))
        .child(header_label_with_min(
            activity_header_label(window),
            palette,
            ACTIVITY_COL_PX,
        ))
        .child(header_label_with_min("Receive", palette, 92.))
        .child(header_label_with_min("Send", palette, 92.))
        .child(header_label_with_min("Total", palette, 100.))
        .child(
            rect()
                .width(AdapterTableLayout::chevron())
                .height(Size::px(1.)),
        )
        .into()
}

fn header_label_with_min(text: &'static str, palette: Palette, width_px: f32) -> Element {
    label()
        .text(text)
        .font_size(10.)
        .font_weight(FontWeight::BOLD)
        .color(palette.muted)
        .width(Size::px(width_px))
        .min_width(Size::px(width_px))
        .into()
}

/// Top adapters by traffic; capped on Overview.
pub fn interfaces_for_table<'a>(
    interfaces: &'a [InterfaceStats],
    mode: AdapterTableMode,
) -> Vec<&'a InterfaceStats> {
    let mut ranked: Vec<&InterfaceStats> = interfaces.iter().collect();
    ranked.sort_by(|a, b| {
        b.combined_bps
            .partial_cmp(&a.combined_bps)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    if matches!(mode, AdapterTableMode::OverviewStatic) {
        ranked.truncate(OVERVIEW_STATIC_ADAPTER_ROWS);
    }
    ranked
}

fn overview_adapter_rows(
    snapshot: &NetworkSnapshot,
    connections: &[ConnectionDetail],
    palette: Palette,
    selected: State<Option<String>>,
    window: TimeWindow,
    sample_tick: u64,
    mode: AdapterTableMode,
) -> Element {
    if snapshot.interfaces.is_empty() {
        let network = NetworkSnapshot {
            sample_tick,
            ..NetworkSnapshot::default()
        };
        return label()
            .text(DataHealth::adapter_empty_message(&network))
            .font_size(14.)
            .color(palette.muted)
            .into();
    }

    let interfaces = interfaces_for_table(&snapshot.interfaces, mode);

    let rows: Vec<Element> = interfaces
        .iter()
        .enumerate()
        .map(|(index, iface)| {
            overview_adapter_row(
                iface,
                connections,
                index + 1 == interfaces.len(),
                palette,
                selected,
                window,
                sample_tick,
            )
        })
        .collect();

    if matches!(mode, AdapterTableMode::FullList) {
        return ScrollView::new()
            .height(Size::fill())
            .spacing(0.)
            .children(rows)
            .into();
    }

    rect().vertical().width(Size::fill()).spacing(0.).children(rows).into()
}

fn overview_adapter_row(
    iface: &InterfaceStats,
    connections: &[ConnectionDetail],
    is_last: bool,
    palette: Palette,
    selected: State<Option<String>>,
    window: TimeWindow,
    sample_tick: u64,
) -> Element {
    let iface_name = iface.name.clone();
    let title = adapter_title(&iface.name);
    let hardware = adapter_hardware_hint(&iface.name);
    let iface_conns: Vec<ConnectionDetail> = connections_for_interface(&iface.name, connections)
        .into_iter()
        .cloned()
        .collect();
    let (character, _) = classify_interface(iface, &iface_conns);
    let personality = personality_from_character(character);
    let is_selected = selected.peek().as_ref() == Some(&iface_name);
    let row_bg = if is_selected {
        Color::from_argb(40, palette.receive.r(), palette.receive.g(), palette.receive.b())
    } else {
        palette.panel
    };
    let rx = slice_history(&iface.rx_history, window);
    let tx = slice_history(&iface.tx_history, window);
    let combined = slice_history(&iface.combined_history, window);
    let spark_max_y = sparkline_scale(&rx, &tx, &combined);
    let status_color = if iface.is_active() {
        palette.send
    } else {
        palette.muted
    };
    let row_key = scope_id(&iface.name, ProcessLane::Green);

    let mut row = rect()
        .horizontal()
        .width(Size::fill())
        .padding(Gaps::new(10., 0., 10., 0.))
        .background(row_bg)
        .spacing(8.)
        .key(row_key)
        .on_mouse_up(move |e: Event<MouseEventData>| {
            e.stop_propagation();
            let mut sel = selected.write_unchecked();
            if sel.as_ref() == Some(&iface_name) {
                *sel = None;
            } else {
                *sel = Some(iface_name.clone());
            }
        });

    if !is_last {
        row = row.border(
            Border::new()
                .fill(palette.panel_edge)
                .width(BorderWidth {
                    top: 0.,
                    right: 0.,
                    bottom: 1.,
                    left: 0.,
                }),
        );
    }

    row.child(
            rect()
                .vertical()
                .spacing(4.)
                .width(Size::px(ADAPTER_NAME_COL_PX))
                .min_width(Size::px(ADAPTER_NAME_COL_PX))
                .child(
                    rect()
                        .horizontal()
                        .spacing(6.)
                        .child(status_dot(status_color))
                        .child(
                            label()
                                .text(title)
                                .font_size(13.)
                                .font_weight(FontWeight::BOLD)
                                .color(palette.text),
                        ),
                )
                .child(
                    label()
                        .text(hardware)
                        .font_size(10.)
                        .color(palette.muted),
                )
                .child(
                    rect()
                        .horizontal()
                        .spacing(5.)
                        .child(status_dot(status_color))
                        .child(
                            label()
                                .text(iface.status_label())
                                .font_size(10.)
                                .color(status_color),
                        ),
                ),
        )
        .child(
            rect()
                .width(Size::px(STATUS_COL_PX))
                .min_width(Size::px(STATUS_COL_PX))
                .child(personality_badge(personality, palette)),
        )
        .child(
            rect()
                .width(Size::px(ACTIVITY_COL_PX))
                .min_width(Size::px(ACTIVITY_COL_PX))
                .height(Size::px(SPARKLINE_HEIGHT))
                .min_height(Size::px(SPARKLINE_HEIGHT))
                .overflow(Overflow::Clip)
                .corner_radius(6.)
                .child(
                    canvas(RenderCallback::new(move |ctx| {
                        draw_activity_sparkline(ctx, &rx, &tx, &combined, palette, spark_max_y);
                    }))
                    .width(Size::px(ACTIVITY_COL_PX))
                    .height(Size::px(SPARKLINE_HEIGHT))
                    .key(row_key.wrapping_add(sample_tick)),
                ),
        )
        .child(rate_label(
            format_rate(iface.rx_bps),
            palette.receive,
            92.,
        ))
        .child(rate_label(
            format_rate(iface.tx_bps),
            palette.send,
            92.,
        ))
        .child(
            rect()
                .horizontal()
                .width(Size::px(100.))
                .min_width(Size::px(100.))
                .spacing(4.)
                .child(rate_label(
                    format_rate(iface.combined_bps),
                    palette.total,
                    84.,
                ))
                .child(
                    label()
                        .text(if is_selected { "v" } else { ">" })
                        .font_size(12.)
                        .color(palette.muted)
                        .width(AdapterTableLayout::chevron()),
                ),
        )
        .into()
}

fn rate_label(text: String, color: Color, width_px: f32) -> Element {
    label()
        .text(text)
        .font_size(12.)
        .font_weight(FontWeight::BOLD)
        .color(color)
        .width(Size::px(width_px))
        .min_width(Size::px(width_px))
        .into()
}

fn status_dot(color: Color) -> Element {
    rect()
        .width(Size::px(7.))
        .height(Size::px(7.))
        .background(color)
        .corner_radius(4.)
        .into()
}

pub fn overview_network_hero(
    snapshot: NetworkSnapshot,
    palette: Palette,
    time_window: State<TimeWindow>,
    window: TimeWindow,
    chart_scales: State<ChartScaleBank>,
) -> Element {
    let mut chart_scales = chart_scales;
    let rx = slice_history(&snapshot.rx_history, window);
    let tx = slice_history(&snapshot.tx_history, window);
    let combined = slice_history(&snapshot.combined_history, window);
    let peak = combined.iter().copied().fold(0.0_f64, f64::max);
    let peak_label = if peak > 0.0 {
        format!("{} Peak Total", format_rate(peak))
    } else {
        "Peak Total —".into()
    };
    let max_y = chart_scales
        .write()
        .hero_y(window, &rx, &tx, &combined);
    let render_max_y = display_chart_max(max_y, &rx, &tx, &combined);
    let chart_key = snapshot.sample_tick;
    let window_key = match window {
        TimeWindow::Sec60 => 0u64,
        TimeWindow::Min5 => 1,
        TimeWindow::Min15 => 2,
    };

    rect()
        .vertical()
        .width(Size::fill())
        .height(Size::px(360.))
        .overflow(Overflow::Clip)
        .background(palette.panel)
        .corner_radius(12.)
        .border(palette.border())
        .padding(Gaps::new_all(12.))
        .spacing(8.)
        .child(overview_legend_row(palette))
        .child(
            rect()
                .horizontal()
                .width(Size::fill())
                .main_align(Alignment::End)
                .child(
                    label()
                        .text(peak_label)
                        .font_size(11.)
                        .font_weight(FontWeight::BOLD)
                        .color(palette.text),
                ),
        )
        .child(
            canvas(RenderCallback::new(move |ctx| {
                draw_network_activity(
                    ctx,
                    &rx,
                    &tx,
                    &combined,
                    palette,
                    window,
                    render_max_y,
                );
            }))
            .width(Size::fill())
            .height(Size::px(HERO_CHART_HEIGHT))
            .key(chart_key.wrapping_add(window_key)),
        )
        .into()
}

fn overview_time_window_picker(
    time_window: State<TimeWindow>,
    active: TimeWindow,
    palette: Palette,
) -> Element {
    rect()
        .child(
            label()
                .text(active.subtitle())
                .font_size(11.)
                .color(palette.muted),
        )
        .on_mouse_up(move |e: Event<MouseEventData>| {
            e.stop_propagation();
            let next = match active {
                TimeWindow::Sec60 => TimeWindow::Min5,
                TimeWindow::Min5 => TimeWindow::Min15,
                TimeWindow::Min15 => TimeWindow::Sec60,
            };
            *time_window.write_unchecked() = next;
        })
        .into()
}

fn overview_legend_row(palette: Palette) -> Element {
    rect()
        .horizontal()
        .width(Size::fill())
        .spacing(16.)
        .main_align(Alignment::End)
        .child(
            label()
                .text("Receive")
                .font_size(10.)
                .color(palette.receive),
        )
        .child(
            label()
                .text("Send")
                .font_size(10.)
                .color(palette.send),
        )
        .child(
            label()
                .text("Total")
                .font_size(10.)
                .color(palette.total),
        )
        .into()
}

#[cfg(test)]
mod tests {
    use freya::prelude::*;
    use freya_testing::prelude::*;

    use super::*;
    use crate::theme::Palette;

    fn sample_iface(name: &str, rx: f64, tx: f64, history: Vec<f64>) -> InterfaceStats {
        InterfaceStats {
            name: name.into(),
            rx_bps: rx,
            tx_bps: tx,
            combined_bps: rx + tx,
            total_rx: 0,
            total_tx: 0,
            consistency: 0.0,
            heavy_consistent: false,
            rx_history: history.clone(),
            tx_history: history.clone(),
            combined_history: history,
        }
    }

    fn sample_snapshot() -> NetworkSnapshot {
        let mut snap = NetworkSnapshot::default();
        snap.interfaces = vec![
            sample_iface("en0", 4800.0, 2400.0, vec![1000.0, 2000.0, 4800.0]),
            sample_iface("en1", 0.0, 0.0, vec![0.0, 0.0]),
        ];
        snap.rx_history = vec![1000.0, 4800.0];
        snap.tx_history = vec![500.0, 2400.0];
        snap.combined_history = vec![1500.0, 7200.0];
        snap
    }

    fn rate_label_layouts(test: &TestingRunner) -> Vec<(String, f32, f32)> {
        test.find_many(|node, element| {
            Label::try_downcast(element).and_then(|label| {
                label.text.contains("B/s").then(|| {
                    let area = node.layout().area;
                    (label.text.to_string(), area.width(), area.height())
                })
            })
        })
    }

    fn canvas_layouts(test: &TestingRunner) -> Vec<(f32, f32)> {
        test.find_many(|node, element| {
            Canvas::try_downcast(element).map(|_| {
                let area = node.layout().area;
                (area.width(), area.height())
            })
        })
    }

    #[test]
    fn overview_static_caps_adapter_rows() {
        let mut snap = NetworkSnapshot::default();
        snap.interfaces = (0..12)
            .map(|i| sample_iface(&format!("if{i}"), i as f64 * 100.0, 0.0, vec![0.0]))
            .collect();
        let ranked = interfaces_for_table(&snap.interfaces, AdapterTableMode::OverviewStatic);
        assert_eq!(ranked.len(), OVERVIEW_STATIC_ADAPTER_ROWS);
        assert_eq!(ranked[0].name, "if11");
        let full = interfaces_for_table(&snap.interfaces, AdapterTableMode::FullList);
        assert_eq!(full.len(), 12);
    }

    fn label_texts(test: &TestingRunner) -> Vec<String> {
        test.find_many(|_, element| {
            Label::try_downcast(element).map(|label| label.text.to_string())
        })
    }

    fn sparkline_canvas_positions(test: &TestingRunner) -> Vec<(f32, f32, f32)> {
        test.find_many(|node, element| {
            Canvas::try_downcast(element).and_then(|_| {
                let area = node.layout().area;
                let h = area.height();
                if h >= SPARKLINE_HEIGHT - 4.0 && h <= SPARKLINE_HEIGHT + 4.0 {
                    Some((area.origin.x, area.width(), area.height()))
                } else {
                    None
                }
            })
        })
    }

    #[test]
    fn overview_adapter_table_shows_activity_column_in_layout() {
        let snapshot = sample_snapshot();
        let palette = Palette::default();

        let mut test = launch_test({
            let snapshot = snapshot.clone();
            move || {
                let selected = use_state(|| None::<String>);
                rect()
                    .width(Size::px(1100.))
                    .height(Size::px(420.))
                    .padding(Gaps::new_all(12.))
                    .child(overview_adapter_table(
                        &snapshot,
                        &[],
                        palette,
                        selected,
                        TimeWindow::Sec60,
                        1,
                        AdapterTableMode::OverviewStatic,
                    ))
            }
        });
        test.sync_and_update();

        let texts = label_texts(&test);
        assert!(
            texts.iter().any(|t| t.contains("Activity")),
            "activity header missing: {texts:?}"
        );
        assert!(
            texts.iter().any(|t| t.contains("Receive")),
            "receive header missing: {texts:?}"
        );

        let sparks = sparkline_canvas_positions(&test);
        assert!(
            sparks.len() >= 2,
            "expected sparkline canvases, got {sparks:?}"
        );
        for (x, width, height) in sparks {
            assert!(
                x < 500.0,
                "sparkline pushed off-screen to the right: x={x} width={width} height={height}"
            );
            assert!(
                x >= ADAPTER_NAME_COL_PX - 24.0,
                "sparkline overlaps name column: x={x}"
            );
            assert!(width >= ACTIVITY_COL_PX - 8.0);
        }
    }

    #[test]
    fn overview_adapter_table_rate_labels_are_not_squished() {
        let snapshot = sample_snapshot();
        let palette = Palette::default();

        let mut test = launch_test({
            let snapshot = snapshot.clone();
            move || {
                let selected = use_state(|| None::<String>);
                rect()
                    .width(Size::px(1100.))
                    .height(Size::px(420.))
                    .padding(Gaps::new_all(12.))
                    .child(overview_adapter_table(
                        &snapshot,
                        &[],
                        palette,
                        selected,
                        TimeWindow::Sec60,
                        1,
                        AdapterTableMode::OverviewStatic,
                    ))
            }
        });
        test.sync_and_update();

        let rates = rate_label_layouts(&test);
        assert!(
            rates.len() >= 4,
            "expected rate labels in adapter rows, got {rates:?}"
        );
        for (text, width, height) in rates {
            assert!(
                width >= MIN_RATE_LABEL_WIDTH,
                "rate label squished: {text:?} width={width} height={height}"
            );
            assert!(
                height <= 24.0,
                "rate label wrapped vertically: {text:?} width={width} height={height}"
            );
        }
    }

    #[test]
    fn overview_renders_hero_and_sparkline_canvases() {
        let snapshot = sample_snapshot();
        let palette = Palette::default();

        let mut test = launch_test({
            let snapshot = snapshot.clone();
            move || {
                let selected = use_state(|| None::<String>);
                let time_window = use_state(TimeWindow::default);
                let chart_scales = use_state(ChartScaleBank::default);
                rect()
                    .width(Size::px(1100.))
                    .height(Size::px(820.))
                    .padding(Gaps::new_all(16.))
                    .vertical()
                    .spacing(12.)
                    .child(overview_network_hero(
                        snapshot.clone(),
                        palette,
                        time_window,
                        TimeWindow::Sec60,
                        chart_scales,
                    ))
                    .child(overview_adapter_table(
                        &snapshot,
                        &[],
                        palette,
                        selected,
                        TimeWindow::Sec60,
                        1,
                        AdapterTableMode::OverviewStatic,
                    ))
            }
        });
        test.sync_and_update();

        let canvases = canvas_layouts(&test);
        assert!(
            canvases
                .iter()
                .any(|(w, h)| *h >= HERO_CHART_HEIGHT - 4.0 && *w >= 400.0),
            "hero chart canvas missing or too small: {canvases:?}"
        );

        let sparklines: Vec<_> = canvases
            .iter()
            .filter(|(w, h)| {
                *h >= SPARKLINE_HEIGHT - 4.0
                    && *h <= SPARKLINE_HEIGHT + 4.0
                    && *w >= MIN_SPARKLINE_WIDTH
            })
            .collect();
        assert!(
            sparklines.len() >= 2,
            "expected sparkline per adapter row, got {canvases:?}"
        );
        for (width, height) in &sparklines {
            assert!(
                *width >= ACTIVITY_COL_PX - 8.0,
                "sparkline squished to edge: width={width} height={height}"
            );
            assert!(
                *height >= SPARKLINE_HEIGHT - 4.0,
                "sparkline too short: width={width} height={height}"
            );
        }
    }

    fn sparkline_canvas_bounds(test: &TestingRunner) -> Vec<(f32, f32, f32, f32)> {
        test.find_many(|node, element| {
            Canvas::try_downcast(element).and_then(|_| {
                let area = node.layout().area;
                let h = area.height();
                if h >= SPARKLINE_HEIGHT - 4.0 && h <= SPARKLINE_HEIGHT + 4.0 {
                    Some((area.origin.x, area.origin.y, area.width(), area.height()))
                } else {
                    None
                }
            })
        })
    }

    #[test]
    fn overview_rendered_sparklines_contain_activity_pixels() {
        use crate::chart_test_harness::decode_png_to_chart;

        let snapshot = sample_snapshot();
        let palette = Palette::default();

        let mut test = launch_test({
            let snapshot = snapshot.clone();
            move || {
                let selected = use_state(|| None::<String>);
                let time_window = use_state(TimeWindow::default);
                let chart_scales = use_state(ChartScaleBank::default);
                rect()
                    .width(Size::px(1100.))
                    .height(Size::px(820.))
                    .padding(Gaps::new_all(16.))
                    .spacing(12.)
                    .child(overview_network_hero(
                        snapshot.clone(),
                        palette,
                        time_window,
                        TimeWindow::Sec60,
                        chart_scales,
                    ))
                    .child(overview_adapter_table(
                        &snapshot,
                        &[],
                        palette,
                        selected,
                        TimeWindow::Sec60,
                        1,
                        AdapterTableMode::OverviewStatic,
                    ))
            }
        });
        test.sync_and_update();

        let bounds = sparkline_canvas_bounds(&test);
        assert!(
            bounds.len() >= 2,
            "expected sparkline canvases in layout: {bounds:?}"
        );

        let png = test.render();
        let chart = decode_png_to_chart(png.as_bytes());

        let mut active_pixels = 0usize;
        for (x, y, w, h) in bounds {
            let x0 = (x + w * 0.4) as i32;
            let x1 = (x + w - 2.0) as i32;
            let y0 = (y + 4.0) as i32;
            let y1 = (y + h - 10.0) as i32;
            let reference = chart.rgb_at(x0, y0 + 1).unwrap_or((0, 0, 0));
            active_pixels += chart.count_pixels_differing_from(x0, y0, x1, y1, reference, 8);
        }

        assert!(
            active_pixels > 80,
            "rendered overview sparklines should contain visible activity pixels, got {active_pixels}"
        );
    }

    #[test]
    fn overview_sparkline_scale_handles_bytes_per_second_traffic() {
        let rx = vec![0.0, 60.0];
        let tx = vec![0.0, 0.0];
        let combined = vec![0.0, 60.0];
        let scale = sparkline_scale(&rx, &tx, &combined);
        assert!(
            scale < crate::charts::MIN_CHART_SCALE,
            "adapter sparkline must not use hero floor for B/s traffic: {scale}"
        );
    }

    #[test]
    fn overview_shows_health_banner_when_degraded() {
        let palette = Palette::default();
        let mut test = launch_test({
            move || {
                overview_health_banner(
                    "Connection details unavailable (nettop failed). Adapter totals still update.",
                    palette,
                )
            }
        });
        test.sync_and_update();

        let labels: Vec<String> = test.find_many(|_, element| {
            Label::try_downcast(element).map(|label| label.text.to_string())
        });
        assert!(
            labels
                .iter()
                .any(|text| text.contains("nettop failed")),
            "expected degraded health banner text, got {labels:?}"
        );
    }

    #[test]
    fn overview_empty_adapters_use_health_copy() {
        let palette = Palette::default();
        let snapshot = NetworkSnapshot::default();
        let mut test = launch_test({
            move || {
                let selected = use_state(|| None::<String>);
                overview_adapter_table(
                    &snapshot,
                    &[],
                    palette,
                    selected,
                    TimeWindow::Sec60,
                    0,
                    AdapterTableMode::OverviewStatic,
                )
            }
        });
        test.sync_and_update();

        let labels: Vec<String> = test.find_many(|_, element| {
            Label::try_downcast(element).map(|label| label.text.to_string())
        });
        assert!(
            labels
                .iter()
                .any(|text| text.contains("Waiting for first adapter sample")),
            "expected first-sample empty adapter copy, got {labels:?}"
        );
    }
}

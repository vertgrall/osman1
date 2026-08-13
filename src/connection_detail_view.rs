//! Full-screen connection drill-down with traffic waveform.

use freya::prelude::*;

use crate::charts::{draw_network_activity, sparkline_scale, ChartScaleBank, MIN_CHART_SCALE};
use crate::detail::ConnectionDetail;
use crate::parse::DataSource;
use crate::rate_tracker::{rate_for_connection, RateTracker};
use crate::theme::{format_rate, format_total, Palette};
use crate::time_window::{slice_history, TimeWindow};

const DETAIL_LABEL_WIDTH: f32 = 120.;

/// Y-axis scale for the connection detail chart — must stay below hero floor for B/s traffic.
pub fn connection_chart_scale(rx: &[f64], tx: &[f64], combined: &[f64]) -> f64 {
    sparkline_scale(rx, tx, combined)
}

pub fn connection_detail_screen(
    conn: ConnectionDetail,
    live_rates: &[crate::rate_tracker::LiveConnectionRate],
    rate_tracker: RateTracker,
    selected_connection: State<Option<ConnectionDetail>>,
    palette: Palette,
    sample_tick: u64,
    mut chart_scales: State<ChartScaleBank>,
) -> Element {
    let live = rate_for_connection(live_rates, conn.id);
    let history = rate_tracker.connection_history(conn.id);
    let session = rate_tracker.session_age(conn.id);
    let session_label = session
        .map(|d| format!("{}m {}s", d.as_secs() / 60, d.as_secs() % 60))
        .unwrap_or_else(|| "—".into());

    let (rx_hist, tx_hist, combined_hist) = history
        .map(|h| (h.rx, h.tx, h.combined))
        .unwrap_or_default();

    let window = TimeWindow::Sec60;
    let rx = slice_history(&rx_hist, window);
    let tx = slice_history(&tx_hist, window);
    let combined = slice_history(&combined_hist, window);
    let locked = chart_scales
        .write()
        .detail_y(conn.id.0, window, &rx, &tx, &combined);
    let render_max_y = locked;
    let peak = combined.iter().copied().fold(0.0_f64, f64::max);
    let peak_label = if peak > 0.0 {
        format!("{} peak", format_rate(peak))
    } else {
        "No recent traffic".into()
    };

    let live_rx = live.map(|r| r.rx_bps).unwrap_or(0.0);
    let live_tx = live.map(|r| r.tx_bps).unwrap_or(0.0);
    let live_total = live.map(|r| r.combined_bps()).unwrap_or(0.0);

    rect()
        .vertical()
        .expanded()
        .background(palette.bg)
        .padding(Gaps::new_all(16.))
        .spacing(12.)
        .child(connection_detail_header(
            &conn,
            selected_connection,
            palette,
        ))
        .child(connection_traffic_hero(
            &rx,
            &tx,
            &combined,
            palette,
            window,
            render_max_y,
            peak_label,
            sample_tick,
        ))
        .child(
            ScrollView::new()
                .width(Size::fill())
                .height(Size::flex(1.))
                .child(
                    rect()
                        .vertical()
                        .width(Size::fill())
                        .spacing(12.)
                        .child(connection_details_card(&conn, session_label, palette))
                        .child(connection_stats_card(
                            &conn,
                            live_rx,
                            live_tx,
                            live_total,
                            palette,
                        )),
                ),
        )
        .into()
}

fn connection_detail_header(
    conn: &ConnectionDetail,
    mut selected_connection: State<Option<ConnectionDetail>>,
    palette: Palette,
) -> Element {
    rect()
        .vertical()
        .width(Size::fill())
        .spacing(8.)
        .child(
            Button::new()
                .on_press(move |_| {
                    selected_connection.set(None);
                })
                .child(
                    label()
                        .text("< Connections")
                        .font_size(12.)
                        .color(palette.receive),
                ),
        )
        .child(
            rect()
                .vertical()
                .spacing(2.)
                .child(
                    label()
                        .text(format!("{} · pid {}", conn.process_name, conn.pid))
                        .font_size(18.)
                        .font_weight(FontWeight::BOLD)
                        .color(palette.title),
                )
                .child(
                    label()
                        .text(conn.remote_label())
                        .font_size(13.)
                        .color(palette.muted),
                ),
        )
        .into()
}

fn connection_traffic_hero(
    rx: &[f64],
    tx: &[f64],
    combined: &[f64],
    palette: Palette,
    window: TimeWindow,
    max_y: f64,
    peak_label: String,
    sample_tick: u64,
) -> Element {
    rect()
        .vertical()
        .width(Size::fill())
        .height(Size::px(320.))
        .overflow(Overflow::Clip)
        .background(palette.panel)
        .corner_radius(12.)
        .border(palette.border())
        .padding(Gaps::new_all(12.))
        .spacing(8.)
        .child(
            rect()
                .vertical()
                .width(Size::fill())
                .spacing(4.)
                .child(
                    label()
                        .text("Connection traffic")
                        .font_size(16.)
                        .font_weight(FontWeight::BOLD)
                        .color(palette.text),
                )
                .child(
                    label()
                        .text(window.subtitle())
                        .font_size(11.)
                        .color(palette.muted),
                )
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
                                .color(palette.text)
                                .width(Size::px(96.)),
                        ),
                ),
        )
        .child(connection_legend_row(palette))
        .child(
            canvas(RenderCallback::new({
                let rx = rx.to_vec();
                let tx = tx.to_vec();
                let combined = combined.to_vec();
                move |ctx| {
                    draw_network_activity(ctx, &rx, &tx, &combined, palette, window, max_y);
                }
            }))
            .width(Size::fill())
            .height(Size::px(240.))
            .key(sample_tick),
        )
        .into()
}

fn connection_legend_row(palette: Palette) -> Element {
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

fn connection_details_card(
    conn: &ConnectionDetail,
    session_label: String,
    palette: Palette,
) -> Element {
    let status_color = if conn.state.contains("ESTABLISHED") || conn.state.contains("LISTEN") {
        palette.send
    } else {
        palette.muted
    };

    rect()
        .vertical()
        .width(Size::fill())
        .spacing(6.)
        .padding(Gaps::new_all(12.))
        .background(palette.panel)
        .corner_radius(12.)
        .border(palette.border())
        .child(section_title("Details", palette))
        .child(kv_row("Remote", conn.remote_label(), palette, None))
        .child(kv_row("Local", conn.local_label(), palette, None))
        .child(kv_row("Interface", conn.interface.clone(), palette, None))
        .child(kv_row("Protocol", conn.transport.to_uppercase(), palette, None))
        .child(kv_row("Role", conn.role_label().into(), palette, None))
        .child(kv_row(
            "Direction",
            conn.direction_label().into(),
            palette,
            None,
        ))
        .child(kv_row("Link", conn.state.clone(), palette, Some(status_color)))
        .child(kv_row("Session", session_label, palette, None))
        .child(kv_row(
            "Source",
            data_source_label(conn.source),
            palette,
            None,
        ))
        .into()
}

fn connection_stats_card(
    conn: &ConnectionDetail,
    live_rx: f64,
    live_tx: f64,
    live_total: f64,
    palette: Palette,
) -> Element {
    rect()
        .vertical()
        .width(Size::fill())
        .spacing(6.)
        .padding(Gaps::new_all(12.))
        .background(palette.panel)
        .corner_radius(12.)
        .border(palette.border())
        .child(section_title("Statistics", palette))
        .child(kv_row(
            "Live receive",
            format_rate(live_rx),
            palette,
            Some(palette.receive),
        ))
        .child(kv_row(
            "Live send",
            format_rate(live_tx),
            palette,
            Some(palette.send),
        ))
        .child(kv_row(
            "Live total",
            format_rate(live_total),
            palette,
            Some(palette.total),
        ))
        .child(kv_row(
            "Bytes received",
            format_total(conn.rx_bytes),
            palette,
            None,
        ))
        .child(kv_row(
            "Bytes sent",
            format_total(conn.tx_bytes),
            palette,
            None,
        ))
        .child(kv_row(
            "Bytes total",
            format_total(conn.combined_bytes()),
            palette,
            None,
        ))
        .into()
}

fn section_title(title: &'static str, palette: Palette) -> Element {
    label()
        .text(title)
        .font_size(11.)
        .font_weight(FontWeight::BOLD)
        .color(palette.muted)
        .into()
}

fn kv_row(
    label_text: &'static str,
    value: String,
    palette: Palette,
    value_color: Option<Color>,
) -> Element {
    rect()
        .horizontal()
        .width(Size::fill())
        .spacing(8.)
        .child(
            label()
                .text(label_text)
                .font_size(11.)
                .color(palette.muted)
                .width(Size::px(DETAIL_LABEL_WIDTH)),
        )
        .child(
            label()
                .text(value)
                .font_size(11.)
                .font_weight(FontWeight::BOLD)
                .color(value_color.unwrap_or(palette.text))
                .width(Size::flex(1.)),
        )
        .into()
}

fn data_source_label(source: DataSource) -> String {
    match source {
        DataSource::Nettop => "nettop".into(),
        DataSource::Lsof => "lsof".into(),
        DataSource::Merged => "merged".into(),
    }
}

/// Compact detail pane for the split inspector (no back button).
pub fn connection_detail_pane(
    conn: ConnectionDetail,
    live_rates: &[crate::rate_tracker::LiveConnectionRate],
    rate_tracker: RateTracker,
    selected_connection: State<Option<ConnectionDetail>>,
    palette: Palette,
    sample_tick: u64,
    mut chart_scales: State<ChartScaleBank>,
) -> Element {
    let live = rate_for_connection(live_rates, conn.id);
    let history = rate_tracker.connection_history(conn.id);
    let (rx_hist, tx_hist, combined_hist) = history
        .map(|h| (h.rx, h.tx, h.combined))
        .unwrap_or_default();
    let window = TimeWindow::Sec60;
    let rx = slice_history(&rx_hist, window);
    let tx = slice_history(&tx_hist, window);
    let combined = slice_history(&combined_hist, window);
    let render_max_y = chart_scales
        .write()
        .detail_y(conn.id.0, window, &rx, &tx, &combined);
    let live_rx = live.map(|r| r.rx_bps).unwrap_or(0.0);
    let live_tx = live.map(|r| r.tx_bps).unwrap_or(0.0);
    let live_total = live.map(|r| r.combined_bps()).unwrap_or(0.0);
    let footer = format!(
        "{} · {} · last 60s",
        crate::adapters::adapter_title(&conn.interface),
        conn.remote_label()
    );

    rect()
        .vertical()
        .width(Size::percent(54.))
        .height(Size::fill())
        .background(palette.panel)
        .corner_radius(12.)
        .border(palette.border())
        .padding(Gaps::new_all(12.))
        .spacing(10.)
        .child(
            rect()
                .vertical()
                .spacing(2.)
                .child(
                    label()
                        .text(conn.remote_label())
                        .font_size(16.)
                        .font_weight(FontWeight::BOLD)
                        .color(palette.title),
                )
                .child(
                    label()
                        .text(format!(
                            "{} · pid {} · {}",
                            conn.process_name, conn.pid, conn.transport
                        ))
                        .font_size(11.)
                        .color(palette.muted),
                ),
        )
        .child(
            canvas(RenderCallback::new({
                let rx = rx.clone();
                let tx = tx.clone();
                let combined = combined.clone();
                move |ctx| {
                    draw_network_activity(ctx, &rx, &tx, &combined, palette, window, render_max_y);
                }
            }))
            .width(Size::fill())
            .height(Size::px(160.))
            .key(sample_tick.wrapping_add(conn.id.0)),
        )
        .child(
            rect()
                .horizontal()
                .width(Size::fill())
                .spacing(12.)
                .child(compact_stat("Receive", format_rate(live_rx), palette.receive, palette))
                .child(compact_stat("Send", format_rate(live_tx), palette.send, palette))
                .child(compact_stat(
                    "Total",
                    format_rate(live_total),
                    palette.total,
                    palette,
                )),
        )
        .child(
            ScrollView::new()
                .height(Size::flex(1.))
                .child(
                    rect()
                        .vertical()
                        .spacing(6.)
                        .child(kv_row("Local", conn.local_label(), palette, None))
                        .child(kv_row("Remote", conn.remote_label(), palette, None))
                        .child(kv_row("State", conn.state.clone(), palette, None))
                        .child(kv_row(
                            "Source",
                            data_source_label(conn.source),
                            palette,
                            None,
                        )),
                ),
        )
        .child(
            label()
                .text(footer)
                .font_size(10.)
                .color(palette.muted),
        )
        .into()
}

fn compact_stat(label_text: &str, value: String, color: Color, palette: Palette) -> Element {
    let label_text = label_text.to_string();
    rect()
        .vertical()
        .spacing(2.)
        .child(
            label()
                .text(label_text)
                .font_size(10.)
                .color(palette.muted),
        )
        .child(
            label()
                .text(value)
                .font_size(14.)
                .font_weight(FontWeight::BOLD)
                .color(color),
        )
        .into()
}

const MIN_PEAK_LABEL_WIDTH: f32 = 60.0;

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use freya::prelude::*;
    use freya_testing::prelude::*;

    use super::*;
    use crate::parse::{ConnectionId, Direction, SocketRole};
    use crate::rate_tracker::RateTracker;
    use crate::theme::Palette;

    fn sample_conn() -> ConnectionDetail {
        ConnectionDetail {
            id: ConnectionId(42),
            process_name: "Safari".into(),
            pid: 1001,
            interface: "en0".into(),
            protocol: "tcp".into(),
            transport: "tcp".into(),
            endpoint: "93.184.216.34:443".into(),
            state: "ESTABLISHED".into(),
            local_host: "10.0.0.2".into(),
            local_port: Some(50123),
            remote_host: "93.184.216.34".into(),
            remote_port: Some(443),
            role: SocketRole::Established,
            direction: Direction::Outbound,
            remote_is_private: false,
            remote_is_loopback: false,
            rx_bytes: 4096,
            tx_bytes: 2048,
            source: DataSource::Nettop,
        }
    }

    fn label_texts(test: &TestingRunner) -> Vec<String> {
        test.find_many(|_, element| {
            Label::try_downcast(element).map(|label| label.text.to_string())
        })
    }

    fn canvas_sizes(test: &TestingRunner) -> Vec<(f32, f32)> {
        test.find_many(|node, element| {
            Canvas::try_downcast(element).map(|_| {
                let area = node.layout().area;
                (area.width(), area.height())
            })
        })
    }

    fn peak_label_layouts(test: &TestingRunner) -> Vec<(String, f32, f32)> {
        test.find_many(|node, element| {
            Label::try_downcast(element).and_then(|label| {
                label.text.contains("peak").then(|| {
                    let area = node.layout().area;
                    (label.text.to_string(), area.width(), area.height())
                })
            })
        })
    }

    fn tracker_with_low_traffic(conn: &ConnectionDetail) -> RateTracker {
        let mut tracker = RateTracker::default();
        let interval = Duration::from_secs(1);
        tracker.update(&[conn.clone()], interval);
        tracker.update(
            &[ConnectionDetail {
                rx_bytes: conn.rx_bytes.saturating_add(72),
                tx_bytes: conn.tx_bytes,
                ..conn.clone()
            }],
            interval,
        );
        tracker
    }

    #[test]
    fn connection_chart_scale_visible_for_bytes_per_second_traffic() {
        let rx = vec![0.0, 72.0];
        let tx = vec![0.0, 0.0];
        let combined = vec![0.0, 72.0];
        let scale = connection_chart_scale(&rx, &tx, &combined);
        assert!(
            scale < MIN_CHART_SCALE,
            "72 B/s traffic must not use 512 B/s hero floor: {scale}"
        );
    }

    #[test]
    fn connection_detail_peak_label_not_squished_at_app_width() {
        let conn = sample_conn();
        let palette = Palette::default();
        let tracker = tracker_with_low_traffic(&conn);

        let mut test = launch_test({
            let conn = conn.clone();
            let tracker = tracker.clone();
            move || {
                let selected = use_state(|| Some(conn.clone()));
                let chart_scales = use_state(ChartScaleBank::default);
                rect()
                    .width(Size::px(1100.))
                    .height(Size::px(720.))
                    .child(connection_detail_screen(
                        conn.clone(),
                        &[],
                        tracker.clone(),
                        selected,
                        palette,
                        1,
                        chart_scales,
                    ))
            }
        });
        test.sync_and_update();

        let peaks = peak_label_layouts(&test);
        assert_eq!(peaks.len(), 1, "expected one peak label, got {peaks:?}");
        let (text, width, height) = &peaks[0];
        assert!(
            *height <= 24.0,
            "peak label wrapped vertically: {text:?} width={width} height={height}"
        );
        assert!(
            *width >= MIN_PEAK_LABEL_WIDTH,
            "peak label squished: {text:?} width={width} height={height}"
        );
    }

    #[test]
    fn connection_detail_renders_traffic_chart_and_cards() {
        let conn = sample_conn();
        let palette = Palette::default();

        let mut test = launch_test({
            let conn = conn.clone();
            move || {
                let selected = use_state(|| Some(conn.clone()));
                let chart_scales = use_state(ChartScaleBank::default);
                connection_detail_screen(
                    conn.clone(),
                    &[],
                    RateTracker::default(),
                    selected,
                    palette,
                    1,
                    chart_scales,
                )
            }
        });
        test.sync_and_update();

        let texts = label_texts(&test);
        assert!(
            texts.iter().any(|t| t.contains("< Connections")),
            "missing back link: {texts:?}"
        );
        assert!(
            texts.iter().any(|t| t.contains("Connection traffic")),
            "missing hero title: {texts:?}"
        );
        assert!(
            texts.iter().any(|t| t.contains("Safari")),
            "missing process title: {texts:?}"
        );
        assert!(
            texts.iter().any(|t| t.contains("Details")),
            "missing details card: {texts:?}"
        );
        assert!(
            texts.iter().any(|t| t.contains("Statistics")),
            "missing stats card: {texts:?}"
        );

        let canvases = canvas_sizes(&test);
        assert_eq!(canvases.len(), 1, "expected one traffic canvas");
        let (width, height) = canvases[0];
        assert!(width >= 400.0, "chart too narrow: {width}");
        assert!(height >= 200.0, "chart too short: {height}");
    }

    #[test]
    fn connection_row_click_opens_detail_screen() {
        let conn = sample_conn();
        let palette = Palette::default();

        let mut test = launch_test({
            let conn = conn.clone();
            move || {
                let mut selected = use_state(|| None::<ConnectionDetail>);
                let chart_scales = use_state(ChartScaleBank::default);
                let show_detail = selected.read().is_some();
                rect()
                    .width(Size::px(1100.))
                    .height(Size::px(720.))
                    .background(palette.bg)
                    .child(if show_detail {
                        connection_detail_screen(
                            selected.read().clone().unwrap(),
                            &[],
                            RateTracker::default(),
                            selected,
                            palette,
                            1,
                            chart_scales,
                        )
                    } else {
                        rect()
                            .width(Size::fill())
                            .padding(Gaps::new_all(8.))
                            .on_mouse_up({
                                let conn = conn.clone();
                                move |_| {
                                    selected.set(Some(conn.clone()));
                                }
                            })
                            .child(
                                label()
                                    .text(format!("{} · open detail", conn.process_name))
                                    .font_size(12.)
                                    .color(palette.text),
                            )
                            .into()
                    })
            }
        });
        test.sync_and_update();

        let click_at = test
            .find(|node, element| {
                Label::try_downcast(element).and_then(|label| {
                    label.text.contains("open detail").then(|| {
                        let area = node.layout().area;
                        (
                            (area.origin.x + area.width() / 2.) as f64,
                            (area.origin.y + area.height() / 2.) as f64,
                        )
                    })
                })
            })
            .expect("connection row");
        test.click_cursor(click_at);
        test.sync_and_update();

        let texts = label_texts(&test);
        assert!(
            texts.iter().any(|t| t.contains("Connection traffic")),
            "detail screen did not open after click: {texts:?}"
        );
    }

    #[test]
    fn connection_detail_traffic_chart_draws_visible_pixels() {
        use crate::chart_test_harness::render_network_activity;
        use crate::time_window::TimeWindow;

        let conn = sample_conn();
        let tracker = tracker_with_low_traffic(&conn);
        let history = tracker.connection_history(conn.id).expect("history");
        let rx = history.rx;
        let tx = history.tx;
        let combined = history.combined;
        let scale = crate::charts::sparkline_scale(&rx, &tx, &combined);
        let palette = Palette::default();
        let fill = palette.chart_fill;

        let chart = render_network_activity(
            640.0,
            280.0,
            &rx,
            &tx,
            &combined,
            palette,
            TimeWindow::Sec60,
            scale,
        );
        let active = chart.count_pixels_differing_from(60, 20, 620, 240, fill, 10);
        assert!(
            active > 40,
            "connection detail chart should paint low-traffic series, got {active} (scale={scale})"
        );
    }

    #[test]
    fn connection_detail_y_axis_labels_are_rate_ticks() {
        let rx = vec![0.0, 36.0, 72.0, 90.0];
        let tx = vec![0.0, 0.0, 12.0, 18.0];
        let combined = vec![0.0, 36.0, 84.0, 108.0];
        let scale = connection_chart_scale(&rx, &tx, &combined);
        let labels = crate::charts::chart_y_labels(scale);
        assert_eq!(labels[0], "0 B/s");
        assert!(labels[1].contains("B/s"), "mid tick: {:?}", labels[1]);
        assert!(labels[2].contains("B/s"), "max tick: {:?}", labels[2]);
        assert_ne!(labels[1], labels[2]);
        assert!(
            scale < crate::charts::MIN_CHART_SCALE,
            "connection Y scale should stay below hero floor: {scale}"
        );
    }
}

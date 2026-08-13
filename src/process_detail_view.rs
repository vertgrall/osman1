//! Full-screen process drill-down with aggregate traffic waveform and socket list.

use freya::prelude::*;

use crate::charts::{draw_network_activity, sparkline_scale};
use crate::detail::{ConnectionDetail, ProcessTraffic};
use crate::rate_tracker::{
    rate_for_connection, rates_for_pid, RateTracker,
};
use crate::theme::{format_rate, format_total, Palette};
use crate::time_window::{slice_history, TimeWindow};

const DETAIL_LABEL_WIDTH: f32 = 120.;

pub fn process_chart_scale(rx: &[f64], tx: &[f64], combined: &[f64]) -> f64 {
    sparkline_scale(rx, tx, combined)
}

pub fn process_detail_screen(
    proc: ProcessTraffic,
    connections: Vec<ConnectionDetail>,
    live_rates: &[crate::rate_tracker::LiveConnectionRate],
    rate_tracker: RateTracker,
    selected_process: State<Option<ProcessTraffic>>,
    selected_connection: State<Option<ConnectionDetail>>,
    app_section: State<crate::AppSection>,
    palette: Palette,
    sample_tick: u64,
) -> Element {
    let pid = proc.pid;
    let filtered: Vec<ConnectionDetail> = connections
        .into_iter()
        .filter(|c| c.pid == pid)
        .collect();

    let history = rate_tracker.process_history(pid, &filtered);
    let (rx_hist, tx_hist, combined_hist) = history
        .map(|h| (h.rx, h.tx, h.combined))
        .unwrap_or_default();

    let window = TimeWindow::Sec60;
    let rx = slice_history(&rx_hist, window);
    let tx = slice_history(&tx_hist, window);
    let combined = slice_history(&combined_hist, window);
    let render_max_y = process_chart_scale(&rx, &tx, &combined);
    let peak = combined.iter().copied().fold(0.0_f64, f64::max);
    let peak_label = if peak > 0.0 {
        format!("{} peak", format_rate(peak))
    } else {
        "No recent traffic".into()
    };

    let (live_rx, live_tx) = rates_for_pid(live_rates, &filtered, pid);
    let live_total = live_rx + live_tx;

    rect()
        .vertical()
        .expanded()
        .background(palette.bg)
        .padding(Gaps::new_all(16.))
        .spacing(12.)
        .child(process_detail_header(
            &proc,
            selected_process,
            palette,
        ))
        .child(process_traffic_hero(
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
                        .child(process_details_card(&proc, filtered.len(), palette))
                        .child(process_stats_card(
                            &proc,
                            live_rx,
                            live_tx,
                            live_total,
                            palette,
                        ))
                        .child(process_connections_card(
                            filtered,
                            live_rates,
                            selected_connection,
                            app_section,
                            palette,
                        )),
                ),
        )
        .into()
}

fn process_detail_header(
    proc: &ProcessTraffic,
    mut selected_process: State<Option<ProcessTraffic>>,
    palette: Palette,
) -> Element {
    rect()
        .vertical()
        .width(Size::fill())
        .spacing(8.)
        .child(
            Button::new()
                .on_press(move |_| {
                    selected_process.set(None);
                })
                .child(
                    label()
                        .text("< Processes")
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
                        .text(proc.name.clone())
                        .font_size(18.)
                        .font_weight(FontWeight::BOLD)
                        .color(palette.title),
                )
                .child(
                    label()
                        .text(format!("pid {} · {} sockets", proc.pid, proc.connection_count))
                        .font_size(13.)
                        .color(palette.muted),
                ),
        )
        .into()
}

fn process_traffic_hero(
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
                        .text("Process traffic")
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
        .child(process_legend_row(palette))
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

fn process_legend_row(palette: Palette) -> Element {
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

fn process_details_card(proc: &ProcessTraffic, socket_count: usize, palette: Palette) -> Element {
    rect()
        .vertical()
        .width(Size::fill())
        .spacing(6.)
        .padding(Gaps::new_all(12.))
        .background(palette.panel)
        .corner_radius(12.)
        .border(palette.border())
        .child(section_title("Details", palette))
        .child(kv_row("Process", proc.name.clone(), palette, None))
        .child(kv_row("PID", proc.pid.to_string(), palette, None))
        .child(kv_row("Sockets", socket_count.to_string(), palette, None))
        .into()
}

fn process_stats_card(
    proc: &ProcessTraffic,
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
            format_total(proc.rx_bytes),
            palette,
            None,
        ))
        .child(kv_row(
            "Bytes sent",
            format_total(proc.tx_bytes),
            palette,
            None,
        ))
        .child(kv_row(
            "Bytes total",
            format_total(proc.combined_bytes()),
            palette,
            None,
        ))
        .into()
}

fn process_connections_card(
    mut connections: Vec<ConnectionDetail>,
    live_rates: &[crate::rate_tracker::LiveConnectionRate],
    selected_connection: State<Option<ConnectionDetail>>,
    app_section: State<crate::AppSection>,
    palette: Palette,
) -> Element {
    connections.sort_by(|a, b| {
        b.combined_bytes()
            .partial_cmp(&a.combined_bytes())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let rows: Vec<Element> = connections
        .into_iter()
        .enumerate()
        .map(|(i, conn)| {
            process_connection_row(conn, live_rates, selected_connection, app_section, palette, i)
        })
        .collect();

    rect()
        .vertical()
        .width(Size::fill())
        .spacing(6.)
        .padding(Gaps::new_all(12.))
        .background(palette.panel)
        .corner_radius(12.)
        .border(palette.border())
        .child(section_title("Connections", palette))
        .child(
            rect()
                .vertical()
                .width(Size::fill())
                .spacing(4.)
                .children(if rows.is_empty() {
                    vec![empty_row("No active sockets for this process.".into(), palette)]
                } else {
                    rows
                }),
        )
        .into()
}

fn process_connection_row(
    conn: ConnectionDetail,
    live_rates: &[crate::rate_tracker::LiveConnectionRate],
    mut selected_connection: State<Option<ConnectionDetail>>,
    mut app_section: State<crate::AppSection>,
    palette: Palette,
    index: usize,
) -> Element {
    let remote = conn.remote_label();
    let local = conn.local_label();
    let transport = conn.transport.clone();
    let state = conn.state.clone();
    let total = conn.combined_bytes();
    let live = rate_for_connection(live_rates, conn.id)
        .map(|r| r.combined_bps())
        .unwrap_or(0.0);
    let conn_for_select = conn.clone();
    let bg = if index % 2 == 0 {
        palette.panel
    } else {
        palette.bg
    };

    rect()
        .horizontal()
        .width(Size::fill())
        .padding(Gaps::new(8., 10., 8., 10.))
        .background(bg)
        .spacing(8.)
        .on_mouse_up(move |e: Event<MouseEventData>| {
            e.stop_propagation();
            selected_connection.set(Some(conn_for_select.clone()));
            app_section.set(crate::AppSection::Connections);
        })
        .children(vec![
            label()
                .text(remote)
                .font_size(11.)
                .font_weight(FontWeight::BOLD)
                .color(palette.text)
                .width(Size::percent(28.))
                .into(),
            label()
                .text(local)
                .font_size(11.)
                .color(palette.muted)
                .width(Size::percent(24.))
                .into(),
            label()
                .text(transport.to_uppercase())
                .font_size(11.)
                .color(palette.muted)
                .width(Size::percent(10.))
                .into(),
            label()
                .text(state)
                .font_size(11.)
                .color(palette.muted)
                .width(Size::percent(14.))
                .into(),
            label()
                .text(format_rate(live))
                .font_size(12.)
                .font_weight(FontWeight::BOLD)
                .color(palette.total)
                .width(Size::percent(12.))
                .into(),
            label()
                .text(format_total(total))
                .font_size(12.)
                .font_weight(FontWeight::BOLD)
                .color(palette.total)
                .width(Size::percent(12.))
                .into(),
        ])
        .into()
}

fn empty_row(message: String, palette: Palette) -> Element {
    rect()
        .padding(Gaps::new_all(12.))
        .child(
            label()
                .text(message)
                .font_size(12.)
                .color(palette.muted),
        )
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

/// Compact detail pane for the split inspector (no back button).
pub fn process_detail_pane(
    proc: ProcessTraffic,
    mut connections: Vec<ConnectionDetail>,
    live_rates: &[crate::rate_tracker::LiveConnectionRate],
    rate_tracker: RateTracker,
    selected_process: State<Option<ProcessTraffic>>,
    selected_connection: State<Option<ConnectionDetail>>,
    app_section: State<crate::AppSection>,
    palette: Palette,
    sample_tick: u64,
    footer: String,
) -> Element {
    let pid = proc.pid;
    let history = rate_tracker.process_history(pid, &connections);
    let (rx_hist, tx_hist, combined_hist) = history
        .map(|h| (h.rx, h.tx, h.combined))
        .unwrap_or_default();
    let window = TimeWindow::Sec60;
    let rx = slice_history(&rx_hist, window);
    let tx = slice_history(&tx_hist, window);
    let combined = slice_history(&combined_hist, window);
    let render_max_y = process_chart_scale(&rx, &tx, &combined);
    let (live_rx, live_tx) = rates_for_pid(live_rates, &connections, pid);
    let live_total = live_rx + live_tx;
    let personality =
        crate::traffic_character::personality_from_history(&combined_hist, live_total);

    connections.sort_by(|a, b| {
        b.combined_bytes()
            .partial_cmp(&a.combined_bytes())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let socket_rows: Vec<Element> = connections
        .iter()
        .take(8)
        .map(|conn| {
            let remote = conn.remote_label();
            let live = rate_for_connection(live_rates, conn.id)
                .map(|r| format_rate(r.combined_bps()))
                .unwrap_or_else(|| "—".into());
            rect()
                .horizontal()
                .width(Size::fill())
                .padding(Gaps::new(6., 0., 6., 0.))
                .child(
                    label()
                        .text(remote)
                        .font_size(10.)
                        .color(palette.text)
                        .width(Size::flex(1.)),
                )
                .child(
                    label()
                        .text(live)
                        .font_size(10.)
                        .font_weight(FontWeight::BOLD)
                        .color(palette.receive),
                )
                .into()
        })
        .collect();

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
                .horizontal()
                .width(Size::fill())
                .cross_align(Alignment::Center)
                .child(
                    rect()
                        .vertical()
                        .spacing(2.)
                        .child(
                            label()
                                .text(proc.name.clone())
                                .font_size(16.)
                                .font_weight(FontWeight::BOLD)
                                .color(palette.title),
                        )
                        .child(
                            label()
                                .text(format!("pid {}", proc.pid))
                                .font_size(11.)
                                .color(palette.muted),
                        ),
                )
                .child(
                    rect()
                        .width(Size::fill())
                        .main_align(Alignment::End)
                        .child(crate::instrument_ui::personality_badge(
                            personality,
                            palette,
                        )),
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
            .key(sample_tick.wrapping_add(pid as u64)),
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
            label()
                .text(format!("Sockets ({})", connections.len()))
                .font_size(10.)
                .font_weight(FontWeight::BOLD)
                .color(palette.muted),
        )
        .child(
            ScrollView::new()
                .height(Size::px(120.))
                .spacing(2.)
                .children(if socket_rows.is_empty() {
                    vec![empty_row("No active sockets.".into(), palette)]
                } else {
                    socket_rows
                }),
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

#[cfg(test)]
mod tests {
    use freya::prelude::*;
    use freya_testing::prelude::*;

    use super::*;
    use crate::parse::{ConnectionId, DataSource, Direction, SocketRole};
    use crate::rate_tracker::RateTracker;
    use crate::theme::Palette;

    fn sample_proc() -> ProcessTraffic {
        ProcessTraffic {
            name: "Safari".into(),
            pid: 1001,
            rx_bytes: 8192,
            tx_bytes: 4096,
            connection_count: 1,
        }
    }

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

    #[test]
    fn process_detail_renders_title_and_chart() {
        let proc = sample_proc();
        let conn = sample_conn();
        let palette = Palette::default();

        let mut test = launch_test({
            let proc = proc.clone();
            let conn = conn.clone();
            move || {
                let selected_process = use_state(|| Some(proc.clone()));
                let selected_connection = use_state(|| None::<ConnectionDetail>);
                let app_section = use_state(|| crate::AppSection::Processes);
                process_detail_screen(
                    proc.clone(),
                    vec![conn.clone()],
                    &[],
                    RateTracker::default(),
                    selected_process,
                    selected_connection,
                    app_section,
                    palette,
                    1,
                )
            }
        });
        test.sync_and_update();

        let texts = label_texts(&test);
        assert!(
            texts.iter().any(|t| t.contains("< Processes")),
            "missing back link: {texts:?}"
        );
        assert!(
            texts.iter().any(|t| t == "Safari"),
            "missing process title: {texts:?}"
        );
        assert!(
            texts.iter().any(|t| t.contains("Process traffic")),
            "missing hero title: {texts:?}"
        );
        assert!(
            texts.iter().any(|t| t.contains("Connections")),
            "missing connections card: {texts:?}"
        );
    }

    #[test]
    fn process_row_click_opens_detail_screen() {
        let proc = sample_proc();
        let palette = Palette::default();

        let mut test = launch_test({
            let proc = proc.clone();
            move || {
                let mut selected = use_state(|| None::<ProcessTraffic>);
                let selected_connection = use_state(|| None::<ConnectionDetail>);
                let app_section = use_state(|| crate::AppSection::Processes);
                let show_detail = selected.read().is_some();
                rect()
                    .width(Size::px(1100.))
                    .height(Size::px(720.))
                    .background(palette.bg)
                    .child(if show_detail {
                        process_detail_screen(
                            selected.read().clone().unwrap(),
                            vec![],
                            &[],
                            RateTracker::default(),
                            selected,
                            selected_connection,
                            app_section,
                            palette,
                            1,
                        )
                    } else {
                        rect()
                            .width(Size::fill())
                            .padding(Gaps::new_all(8.))
                            .on_mouse_up({
                                let proc = proc.clone();
                                move |_| {
                                    selected.set(Some(proc.clone()));
                                }
                            })
                            .child(
                                label()
                                    .text(format!("{} · open detail", proc.name))
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
            .expect("process row");
        test.click_cursor(click_at);
        test.sync_and_update();

        let texts = label_texts(&test);
        assert!(
            texts.iter().any(|t| t.contains("Process traffic")),
            "detail screen did not open after click: {texts:?}"
        );
    }
}

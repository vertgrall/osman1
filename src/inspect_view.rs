//! Split inspector — Processes | Connections list + detail pane (instrument mock).

use freya::prelude::*;

use crate::adapters::adapter_title;
use crate::charts::{draw_activity_sparkline, sparkline_scale, ChartScaleBank};
use crate::connection_detail_view::connection_detail_pane;
use crate::detail::{ConnectionDetail, ProcessTraffic};
use crate::instrument_ui::{inspect_mode_toggle, personality_badge, process_letter_mark};
use crate::process_detail_view::process_detail_pane;
use crate::rate_tracker::{rate_for_connection, rates_for_pid, RateTracker};
use crate::theme::{format_rate, format_total, Palette};
use crate::time_window::{slice_history, TimeWindow};
use crate::traffic_character::personality_from_history;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InspectMode {
    Processes,
    Connections,
}

pub fn inspect_screen(
    mode: InspectMode,
    processes: Vec<ProcessTraffic>,
    connections: Vec<ConnectionDetail>,
    live_rates: Vec<crate::rate_tracker::LiveConnectionRate>,
    rate_tracker: RateTracker,
    proc_filter: String,
    process_filter: State<String>,
    conn_filter: String,
    connection_filter: State<String>,
    selected_process: State<Option<ProcessTraffic>>,
    selected_proc: Option<ProcessTraffic>,
    selected_connection: State<Option<ConnectionDetail>>,
    selected_conn: Option<ConnectionDetail>,
    app_section: State<crate::AppSection>,
    palette: Palette,
    sample_tick: u64,
    chart_scales: State<ChartScaleBank>,
    on_processes_tab: impl FnMut(Event<MouseEventData>) + 'static + Clone,
    on_connections_tab: impl FnMut(Event<MouseEventData>) + 'static + Clone,
) -> Element {
    let title = match mode {
        InspectMode::Processes => "Processes",
        InspectMode::Connections => "Connections",
    };

    rect()
        .vertical()
        .expanded()
        .padding(Gaps::new_all(16.))
        .spacing(12.)
        .child(
            rect()
                .horizontal()
                .width(Size::fill())
                .cross_align(Alignment::Center)
                .child(
                    label()
                        .text(title)
                        .font_size(20.)
                        .font_weight(FontWeight::BOLD)
                        .color(palette.title),
                )
                .child(
                    rect()
                        .width(Size::fill())
                        .main_align(Alignment::Center)
                        .child(inspect_shared_filter(
                            mode,
                            process_filter,
                            connection_filter,
                            palette,
                        )),
                )
                .child(inspect_mode_toggle(
                    mode == InspectMode::Processes,
                    on_processes_tab,
                    on_connections_tab,
                    palette,
                )),
        )
        .child(
            rect()
                .horizontal()
                .expanded()
                .spacing(12.)
                .child(match mode {
                    InspectMode::Processes => inspect_process_list(
                        processes.clone(),
                        connections.clone(),
                        live_rates.clone(),
                        rate_tracker.clone(),
                        proc_filter,
                        selected_process,
                        selected_proc.clone(),
                        palette,
                        sample_tick,
                    ),
                    InspectMode::Connections => inspect_connection_list(
                        connections.clone(),
                        live_rates.clone(),
                        conn_filter,
                        selected_connection,
                        selected_conn.clone(),
                        palette,
                    ),
                })
                .child(match mode {
                    InspectMode::Processes => inspect_process_detail_pane(
                        processes,
                        connections,
                        selected_proc.clone(),
                        live_rates,
                        rate_tracker,
                        selected_process,
                        selected_connection,
                        app_section,
                        palette,
                        sample_tick,
                    ),
                    InspectMode::Connections => inspect_connection_detail_pane(
                        connections,
                        selected_conn.clone(),
                        live_rates,
                        rate_tracker,
                        selected_connection,
                        palette,
                        sample_tick,
                        chart_scales,
                    ),
                }),
        )
        .into()
}

fn inspect_shared_filter(
    mode: InspectMode,
    process_filter: State<String>,
    connection_filter: State<String>,
    _palette: Palette,
) -> Element {
    let placeholder = "Filter processes and sockets…";
    let input = match mode {
        InspectMode::Processes => Input::new(process_filter).placeholder(placeholder),
        InspectMode::Connections => Input::new(connection_filter).placeholder(placeholder),
    };
    rect()
        .width(Size::px(420.))
        .child(input.flat().width(Size::fill()))
        .into()
}

fn inspect_process_list(
    mut processes: Vec<ProcessTraffic>,
    connections: Vec<ConnectionDetail>,
    live_rates: Vec<crate::rate_tracker::LiveConnectionRate>,
    rate_tracker: RateTracker,
    filter: String,
    selected_process: State<Option<ProcessTraffic>>,
    selected_proc: Option<ProcessTraffic>,
    palette: Palette,
    sample_tick: u64,
) -> Element {
    processes.sort_by(|a, b| {
        b.combined_bytes()
            .partial_cmp(&a.combined_bytes())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let needle = filter.to_ascii_lowercase();
    let filtered: Vec<_> = processes
        .into_iter()
        .filter(|p| needle.is_empty() || p.name.to_ascii_lowercase().contains(&needle))
        .collect();

    let effective_pid = selected_proc
        .as_ref()
        .map(|p| p.pid)
        .or_else(|| filtered.first().map(|p| p.pid));

    let rows: Vec<Element> = filtered
        .into_iter()
        .enumerate()
        .map(|(i, proc)| {
            inspect_process_row(
                proc,
                &connections,
                &live_rates,
                &rate_tracker,
                palette,
                selected_process,
                effective_pid,
                i,
                sample_tick,
            )
        })
        .collect();

    inspect_list_shell(
        palette,
        vec![
            ("App", 34.),
            ("Character", 14.),
            ("Live", 18.),
            ("Total", 18.),
        ],
        rows,
        "No matching processes.",
    )
}

fn inspect_process_row(
    proc: ProcessTraffic,
    connections: &[ConnectionDetail],
    live_rates: &[crate::rate_tracker::LiveConnectionRate],
    rate_tracker: &RateTracker,
    palette: Palette,
    mut selected_process: State<Option<ProcessTraffic>>,
    effective_pid: Option<u32>,
    index: usize,
    sample_tick: u64,
) -> Element {
    let pid = proc.pid;
    let filtered: Vec<ConnectionDetail> = connections
        .iter()
        .filter(|c| c.pid == pid)
        .cloned()
        .collect();
    let (live_rx, live_tx) = rates_for_pid(live_rates, &filtered, pid);
    let live_total = live_rx + live_tx;
    let history = rate_tracker.process_history(pid, &filtered);
    let combined_hist = history
        .as_ref()
        .map(|h| h.combined.clone())
        .unwrap_or_default();
    let personality = personality_from_history(&combined_hist, live_total);
    let rx = slice_history(
        &history.as_ref().map(|h| h.rx.clone()).unwrap_or_default(),
        TimeWindow::Sec60,
    );
    let tx = slice_history(
        &history.as_ref().map(|h| h.tx.clone()).unwrap_or_default(),
        TimeWindow::Sec60,
    );
    let combined = slice_history(&combined_hist, TimeWindow::Sec60);
    let spark_max = sparkline_scale(&rx, &tx, &combined);
    let proc_for_select = proc.clone();
    let is_selected = effective_pid == Some(pid);
    let bg = if is_selected {
        Color::from_argb(40, palette.receive.r(), palette.receive.g(), palette.receive.b())
    } else if index % 2 == 0 {
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
        .cross_align(Alignment::Center)
        .on_mouse_up(move |e: Event<MouseEventData>| {
            e.stop_propagation();
            selected_process.set(Some(proc_for_select.clone()));
        })
        .child(
            rect()
                .horizontal()
                .spacing(8.)
                .width(Size::percent(34.))
                .cross_align(Alignment::Center)
                .child(process_letter_mark(&proc.name, palette))
                .child(
                    label()
                        .text(proc.name.clone())
                        .font_size(12.)
                        .font_weight(FontWeight::BOLD)
                        .color(palette.text),
                ),
        )
        .child(
            rect()
                .width(Size::percent(14.))
                .child(personality_badge(personality, palette)),
        )
        .child(
            rect()
                .width(Size::px(72.))
                .height(Size::px(28.))
                .overflow(Overflow::Clip)
                .corner_radius(4.)
                .child(
                    canvas(RenderCallback::new(move |ctx| {
                        draw_activity_sparkline(ctx, &rx, &tx, &combined, palette, spark_max);
                    }))
                    .width(Size::px(72.))
                    .height(Size::px(28.))
                    .key(sample_tick.wrapping_add(pid as u64)),
                ),
        )
        .child(
            label()
                .text(format_rate(live_total))
                .font_size(12.)
                .font_weight(FontWeight::BOLD)
                .color(palette.receive)
                .width(Size::percent(18.)),
        )
        .child(
            label()
                .text(format_total(proc.combined_bytes()))
                .font_size(12.)
                .color(palette.muted)
                .width(Size::percent(18.)),
        )
        .into()
}

fn inspect_connection_list(
    mut connections: Vec<ConnectionDetail>,
    live_rates: Vec<crate::rate_tracker::LiveConnectionRate>,
    filter: String,
    selected_connection: State<Option<ConnectionDetail>>,
    selected_conn: Option<ConnectionDetail>,
    palette: Palette,
) -> Element {
    connections.sort_by(|a, b| {
        b.combined_bytes()
            .partial_cmp(&a.combined_bytes())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let needle = filter.to_ascii_lowercase();
    let filtered: Vec<_> = connections
        .into_iter()
        .filter(|c| c.matches_filter(&needle))
        .collect();

    let effective_id = selected_conn
        .as_ref()
        .map(|c| c.id)
        .or_else(|| filtered.first().map(|c| c.id));

    let rows: Vec<Element> = filtered
        .into_iter()
        .enumerate()
        .map(|(i, conn)| {
            inspect_connection_row(
                conn,
                &live_rates,
                palette,
                selected_connection,
                effective_id,
                i,
            )
        })
        .collect();

    inspect_list_shell(
        palette,
        vec![
            ("Process", 22.),
            ("Remote", 28.),
            ("Live", 18.),
            ("Total", 18.),
        ],
        rows,
        "No matching connections.",
    )
}

fn inspect_connection_row(
    conn: ConnectionDetail,
    live_rates: &[crate::rate_tracker::LiveConnectionRate],
    palette: Palette,
    mut selected_connection: State<Option<ConnectionDetail>>,
    effective_id: Option<crate::parse::ConnectionId>,
    index: usize,
) -> Element {
    let live = rate_for_connection(live_rates, conn.id)
        .map(|r| r.combined_bps())
        .unwrap_or(0.0);
    let conn_for_select = conn.clone();
    let is_selected = effective_id == Some(conn.id);
    let bg = if is_selected {
        Color::from_argb(40, palette.receive.r(), palette.receive.g(), palette.receive.b())
    } else if index % 2 == 0 {
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
        })
        .child(
            label()
                .text(conn.process_name.clone())
                .font_size(12.)
                .font_weight(FontWeight::BOLD)
                .color(palette.text)
                .width(Size::percent(22.)),
        )
        .child(
            label()
                .text(conn.remote_label())
                .font_size(12.)
                .color(palette.text)
                .width(Size::percent(28.)),
        )
        .child(
            label()
                .text(format_rate(live))
                .font_size(12.)
                .font_weight(FontWeight::BOLD)
                .color(palette.receive)
                .width(Size::percent(18.)),
        )
        .child(
            label()
                .text(format_total(conn.combined_bytes()))
                .font_size(12.)
                .color(palette.muted)
                .width(Size::percent(18.)),
        )
        .into()
}

fn inspect_list_shell(
    palette: Palette,
    headers: Vec<(&'static str, f32)>,
    rows: Vec<Element>,
    empty: &'static str,
) -> Element {
    rect()
        .vertical()
        .width(Size::percent(46.))
        .height(Size::fill())
        .background(palette.panel)
        .corner_radius(12.)
        .border(palette.border())
        .padding(Gaps::new_all(12.))
        .spacing(6.)
        .child(inspect_list_header(palette, &headers))
        .child(
            ScrollView::new()
                .expanded()
                .spacing(2.)
                .children(if rows.is_empty() {
                    vec![empty_state(empty, palette)]
                } else {
                    rows
                }),
        )
        .into()
}

fn inspect_list_header(palette: Palette, headers: &[(&'static str, f32)]) -> Element {
    rect()
        .horizontal()
        .width(Size::fill())
        .padding(Gaps::new(0., 0., 6., 0.))
        .spacing(8.)
        .children(
            headers
                .iter()
                .map(|(text, pct)| {
                    label()
                        .text(*text)
                        .font_size(10.)
                        .font_weight(FontWeight::BOLD)
                        .color(palette.muted)
                        .width(Size::percent(*pct))
                        .into()
                })
                .collect::<Vec<_>>(),
        )
        .into()
}

fn empty_state(message: &str, palette: Palette) -> Element {
    let message = message.to_string();
    rect()
        .padding(Gaps::new_all(24.))
        .child(
            label()
                .text(message)
                .font_size(12.)
                .color(palette.muted),
        )
        .into()
}

fn inspect_process_detail_pane(
    processes: Vec<ProcessTraffic>,
    connections: Vec<ConnectionDetail>,
    selected_proc: Option<ProcessTraffic>,
    live_rates: Vec<crate::rate_tracker::LiveConnectionRate>,
    rate_tracker: RateTracker,
    selected_process: State<Option<ProcessTraffic>>,
    selected_connection: State<Option<ConnectionDetail>>,
    app_section: State<crate::AppSection>,
    palette: Palette,
    sample_tick: u64,
) -> Element {
    let proc = selected_proc
        .or_else(|| processes.into_iter().next())
        .unwrap_or(ProcessTraffic {
            name: "—".into(),
            pid: 0,
            rx_bytes: 0,
            tx_bytes: 0,
            connection_count: 0,
        });

    if proc.pid == 0 {
        return inspect_empty_pane("Select a process", palette);
    }

    let filtered: Vec<ConnectionDetail> = connections
        .iter()
        .filter(|c| c.pid == proc.pid)
        .cloned()
        .collect();
    let iface = filtered
        .first()
        .map(|c| c.interface.as_str())
        .unwrap_or("en0");
    let footer = format!(
        "{} · {} sockets · last 60s",
        adapter_title(iface),
        filtered.len()
    );

    process_detail_pane(
        proc,
        filtered,
        &live_rates,
        rate_tracker,
        selected_process,
        selected_connection,
        app_section,
        palette,
        sample_tick,
        footer,
    )
}

fn inspect_connection_detail_pane(
    connections: Vec<ConnectionDetail>,
    selected_conn: Option<ConnectionDetail>,
    live_rates: Vec<crate::rate_tracker::LiveConnectionRate>,
    rate_tracker: RateTracker,
    selected_connection: State<Option<ConnectionDetail>>,
    palette: Palette,
    sample_tick: u64,
    chart_scales: State<ChartScaleBank>,
) -> Element {
    let conn = selected_conn.or_else(|| connections.into_iter().next());

    let Some(conn) = conn else {
        return inspect_empty_pane("Select a connection", palette);
    };

    connection_detail_pane(
        conn,
        &live_rates,
        rate_tracker,
        selected_connection,
        palette,
        sample_tick,
        chart_scales,
    )
}

fn inspect_empty_pane(message: &str, palette: Palette) -> Element {
    let message = message.to_string();
    rect()
        .vertical()
        .width(Size::percent(54.))
        .height(Size::fill())
        .background(palette.panel)
        .corner_radius(12.)
        .border(palette.border())
        .main_align(Alignment::Center)
        .cross_align(Alignment::Center)
        .child(
            label()
                .text(message)
                .font_size(13.)
                .color(palette.muted),
        )
        .into()
}

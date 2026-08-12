mod about;
mod about_art;
mod about_assets;
mod adapter_table_layout;
mod adapters;
mod alerts;
mod character_render;
mod character_timeline;
mod clinical_render;
mod connection_detail_view;
mod charts;
#[cfg(test)]
mod about_test_harness;
#[cfg(test)]
mod chart_test_harness;
#[cfg(test)]
mod ui_screenshot_harness;
mod detail;
mod lfo;
mod macos_about_menu;
mod menubar;
mod mock_traffic;
mod network;
mod overview_ui;
mod parse;
mod particles;
mod rate_tracker;
mod theme;
mod time_window;
mod traffic_character;
mod traffic_character_view;

use std::time::{Duration, Instant};

use async_io::Timer;
use freya::prelude::*;

use adapters::scope_id;
use alerts::{alerts_screen, AlertEngine};
use character_render::CharacterScopeBank;
use character_timeline::CharacterTimeline;
use charts::{draw_network_activity, ChartScaleBank, MIN_CHART_SCALE};
use detail::{
    interface_detail_from_traffic, ConnectionDetail, InterfaceDetail, ProcessTraffic,
    TrafficSnapshot,
};
use network::{push_history, InterfaceStats, NetworkSnapshot, NetworkTracker, POLL_INTERVAL};
use rate_tracker::{
    rate_for_connection, rates_for_interface, rates_for_process, LiveConnectionRate, RateTracker,
};
use time_window::{slice_history, TimeWindow};
use sysinfo::Networks;
use theme::{format_rate, format_total, Palette, ProcessLane};
use crate::about::about_content;
use crate::adapter_table_layout::AdapterTableMode;
use connection_detail_view::connection_detail_screen;
use overview_ui::{overview_adapter_table, overview_network_hero};
use traffic_character_view::traffic_character_screen;

#[derive(Clone, Copy, PartialEq, Eq)]
enum AppSection {
    Overview,
    Adapters,
    Processes,
    Connections,
    TrafficCharacter,
    Alerts,
    Settings,
}

fn main() {
    about_assets::preload();

    launch(menubar::with_menubar(
        LaunchConfig::new()
            .with_future(|proxy| async move {
                let proxy = proxy.clone();
                menubar::set_renderer_dispatch(Box::new(move |f| {
                    let _ = proxy.clone().post_callback(move |ctx| f(ctx));
                }));
                async_io::Timer::after(std::time::Duration::from_millis(250)).await;
                macos_about_menu::install();
            })
            .with_window(
            WindowConfig::new(app)
                .with_title("Osman by NT")
                .with_size(1400., 920.)
                .with_min_size(1000., 720.)
                .with_background(Palette::default().bg),
            ),
    ));
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AppBootstrap {
    Live,
    Demo,
}

fn app() -> Element {
    app_with_bootstrap(AppBootstrap::Live)
}

/// README / marketing screenshots — same UI with mock traffic pre-loaded.
pub fn app_demo() -> Element {
    app_with_bootstrap(AppBootstrap::Demo)
}

fn app_with_bootstrap(bootstrap: AppBootstrap) -> Element {
    let is_demo = bootstrap == AppBootstrap::Demo;
    let demo_traffic = is_demo.then(mock_traffic::traffic_snapshot);
    let demo_snapshot = is_demo.then(mock_traffic::network_snapshot);
    let demo_rates = demo_traffic
        .as_ref()
        .map(|t| mock_traffic::live_rates(&t.connections));
    let demo_started = is_demo.then(mock_traffic::demo_started_at);
    let demo_traffic_for_system = demo_traffic.clone();
    let demo_traffic_for_state = demo_traffic;

    let snapshot = use_state(move || demo_snapshot.unwrap_or_default());
    let selected = use_state(|| None::<String>);
    let detail = use_state(|| None::<InterfaceDetail>);
    let anim_time = use_state(|| 0.0f64);
    let character_scopes = use_state(CharacterScopeBank::default);
    let app_started = use_state(move || demo_started.unwrap_or_else(Instant::now));
    let app_section = use_state(|| AppSection::Overview);
    let system_traffic = use_state(move || {
        demo_traffic_for_system
            .as_ref()
            .map(|t| (t.processes.clone(), t.connections.clone()))
            .unwrap_or_default()
    });
    let connection_rates = use_state(move || demo_rates.unwrap_or_default());
    let rate_tracker = use_state(RateTracker::default);
    let time_window = use_state(TimeWindow::default);
    let character_timeline = use_state(CharacterTimeline::default);
    let alert_engine = use_state(|| AlertEngine::new());
    let list_filter = use_state(String::new);
    let chart_scales = use_state(ChartScaleBank::default);
    let traffic_snapshot = use_state(move || demo_traffic_for_state.unwrap_or_default());
    let selected_connection = use_state(|| None::<ConnectionDetail>);

    if !is_demo {
        use_future(move || {
            let mut anim_time = anim_time;
            let app_section = app_section;
            let selected = selected;
            async move {
                let redraw = Platform::get().sender.clone();
                let start = Instant::now();
                loop {
                    Timer::after(Duration::from_millis(16)).await;
                    let section = *app_section.peek();
                    let detail_open = selected.peek().is_some();
                    if !section_needs_animation(section, detail_open) {
                        continue;
                    }
                    *anim_time.write() = start.elapsed().as_secs_f64();
                    redraw(UserEvent::RequestRedraw);
                }
            }
        });

        use_future(move || {
            let mut snapshot = snapshot;
            let system_traffic = system_traffic;
            let connection_rates = connection_rates;
            let rate_tracker = rate_tracker;
            let character_timeline = character_timeline;
            let alert_engine = alert_engine;
            let traffic_snapshot = traffic_snapshot;
            async move {
                let redraw = Platform::get().sender.clone();
                let mut networks = Networks::new_with_refreshed_list();
                let mut tracker = NetworkTracker::default();
                loop {
                    Timer::after(POLL_INTERVAL).await;
                    let previous = snapshot.peek().clone();
                    let traffic = TrafficSnapshot::collect();
                    let connection_count = traffic.connection_count();
                    let mut next = tracker.sample(&mut networks, connection_count);
                    push_history(&mut next, &previous);
                    menubar::update_from_snapshot(&next);
                    *snapshot.write() = next.clone();

                    let rates = rate_tracker
                        .write_unchecked()
                        .update(&traffic.connections, POLL_INTERVAL);

                    character_timeline
                        .write_unchecked()
                        .observe_snapshot(&next, &traffic.connections);
                    alert_engine
                        .write_unchecked()
                        .evaluate(&next, &traffic.connections);

                    *system_traffic.write_unchecked() =
                        (traffic.processes.clone(), traffic.connections.clone());
                    *connection_rates.write_unchecked() = rates;
                    *traffic_snapshot.write_unchecked() = traffic;
                    redraw(UserEvent::RequestRedraw);
                }
            }
        });

        use_future(move || {
            let selected = selected;
            let detail = detail;
            let snapshot = snapshot;
            let traffic_snapshot = traffic_snapshot;
            async move {
                loop {
                    if let Some(name) = selected.peek().clone() {
                        let snap = snapshot.peek().clone();
                        let traffic = traffic_snapshot.peek().clone();
                        let loaded = interface_detail_from_traffic(&name, &snap, &traffic);
                        *detail.write_unchecked() = Some(loaded);
                    } else {
                        *detail.write_unchecked() = None;
                    }
                    Timer::after(POLL_INTERVAL).await;
                }
            }
        });
    }

    let palette = Palette::default();
    let data = snapshot.read().clone();
    let selected_name = selected.read().clone();
    let detail_data = detail.read().clone();
    let (processes, connections) = system_traffic.read().clone();
    let live_rates = connection_rates.read().clone();
    let anim_clock = anim_time;
    let char_scopes = character_scopes;
    let started = *app_started.read();
    let section = *app_section.read();
    let window = *time_window.read();
    let timeline = character_timeline.read().clone();
    let alerts = alert_engine.read();
    let filter = list_filter.read().clone();
    let selected_conn = selected_connection.read().clone();
    let rate_tracker_snap = rate_tracker.read().clone();
    let detail_open = selected_name.is_some();
    let anim_frame = if section_needs_animation(section, detail_open) {
        *anim_time.read()
    } else {
        0.0
    };

    rect()
        .expanded()
        .background(palette.bg)
        .horizontal()
        .spacing(0.)
        .child(sidebar(
            &data,
            palette,
            app_section,
            selected,
            selected_connection,
            started,
            section,
            &alerts,
        ))
        .child(main_content(
            section,
            data,
            palette,
            selected,
            selected_name,
            detail_data,
            processes,
            connections,
            live_rates,
            anim_clock,
            char_scopes,
            time_window,
            window,
            timeline,
            alerts.clone(),
            list_filter,
            filter,
            anim_frame,
            chart_scales,
            selected_connection,
            selected_conn,
            rate_tracker_snap,
        ))
        .into()
}

fn main_content(
    section: AppSection,
    snapshot: NetworkSnapshot,
    palette: Palette,
    selected: State<Option<String>>,
    selected_name: Option<String>,
    detail: Option<InterfaceDetail>,
    processes: Vec<ProcessTraffic>,
    connections: Vec<ConnectionDetail>,
    live_rates: Vec<LiveConnectionRate>,
    anim_clock: State<f64>,
    character_scopes: State<CharacterScopeBank>,
    time_window: State<TimeWindow>,
    window: TimeWindow,
    timeline: CharacterTimeline,
    alerts: AlertEngine,
    list_filter: State<String>,
    filter: String,
    anim_frame: f64,
    chart_scales: State<ChartScaleBank>,
    selected_connection: State<Option<ConnectionDetail>>,
    selected_conn: Option<ConnectionDetail>,
    rate_tracker_snap: RateTracker,
) -> Element {
    match section {
        AppSection::Overview => rect()
            .vertical()
            .expanded()
            .padding(Gaps::new_all(16.))
            .spacing(12.)
            .child(overview_network_hero(
                snapshot.clone(),
                palette,
                time_window,
                window,
                chart_scales,
            ))
            .child(adapter_stack(
                &snapshot,
                snapshot.sample_tick,
                palette,
                selected,
                selected_name,
                detail,
                live_rates,
                window,
                chart_scales,
                AdapterTableMode::OverviewStatic,
            ))
            .into(),
        AppSection::Adapters => rect()
            .vertical()
            .expanded()
            .padding(Gaps::new_all(16.))
            .spacing(12.)
            .child(section_heading("Adapters", palette))
            .child(adapter_stack(
                &snapshot,
                snapshot.sample_tick,
                palette,
                selected,
                selected_name,
                detail,
                live_rates,
                window,
                chart_scales,
                AdapterTableMode::FullList,
            ))
            .into(),
        AppSection::Processes => rect()
            .child(processes_view(
                processes,
                live_rates,
                filter,
                list_filter,
                palette,
                true,
            ))
            .into(),
        AppSection::Connections => {
            if let Some(selected) = selected_conn {
                connections_detail_view(
                    connections,
                    selected,
                    live_rates,
                    rate_tracker_snap,
                    selected_connection,
                    palette,
                    snapshot.sample_tick,
                )
                .into()
            } else {
                rect()
                    .vertical()
                    .expanded()
                    .padding(Gaps::new_all(16.))
                    .spacing(12.)
                    .child(section_heading("Connections", palette))
                    .child(connections_list_view(
                        connections,
                        live_rates,
                        filter,
                        list_filter,
                        palette,
                        selected_connection,
                        true,
                    ))
                    .into()
            }
        }
        AppSection::TrafficCharacter => traffic_character_screen(
            snapshot,
            connections,
            processes,
            live_rates,
            palette,
            anim_clock,
            character_scopes,
            timeline,
            window,
        )
        .into(),
        AppSection::Alerts => rect()
            .vertical()
            .expanded()
            .padding(Gaps::new_all(16.))
            .spacing(12.)
            .child(section_heading("Alerts", palette))
            .child(alerts_screen(&alerts, palette))
            .into(),
        AppSection::Settings => rect()
            .vertical()
            .expanded()
            .padding(Gaps::new_all(16.))
            .spacing(12.)
            .child(section_heading("Settings", palette))
            .child(settings_view(palette))
            .into(),
    }
}

fn section_needs_animation(section: AppSection, detail_open: bool) -> bool {
    matches!(section, AppSection::TrafficCharacter) || detail_open
}

fn adapter_stack(
    snapshot: &NetworkSnapshot,
    sample_tick: u64,
    palette: Palette,
    selected: State<Option<String>>,
    selected_name: Option<String>,
    detail: Option<InterfaceDetail>,
    live_rates: Vec<LiveConnectionRate>,
    window: TimeWindow,
    chart_scales: State<ChartScaleBank>,
    mode: AdapterTableMode,
) -> Element {
    let fill = matches!(mode, AdapterTableMode::FullList);
    let mut stack = rect().vertical().spacing(12.);
    if fill {
        stack = stack.expanded();
    }

    stack
        .child(overview_adapter_table(
            snapshot,
            palette,
            selected,
            window,
            sample_tick,
            mode,
        ))
        .maybe_child(selected_name.map(|name| {
            detail_panel(
                name,
                snapshot,
                detail,
                live_rates,
                palette,
                window,
                chart_scales,
            )
        }))
        .into()
}

fn section_heading(title: &'static str, palette: Palette) -> Element {
    label()
        .text(title)
        .font_size(20.)
        .font_weight(FontWeight::BOLD)
        .color(palette.title)
        .into()
}

fn processes_view(
    mut processes: Vec<ProcessTraffic>,
    live_rates: Vec<LiveConnectionRate>,
    filter: String,
    list_filter: State<String>,
    palette: Palette,
    filter_keys: bool,
) -> Element {
    processes.sort_by(|a, b| {
        b.combined_bytes()
            .partial_cmp(&a.combined_bytes())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let needle = filter.to_ascii_lowercase();
    let rows: Vec<Element> = processes
        .into_iter()
        .filter(|p| needle.is_empty() || p.name.to_ascii_lowercase().contains(&needle))
        .enumerate()
        .map(|(i, proc)| process_row(proc, &live_rates, palette, i))
        .collect();

    let body = panel_shell(
        palette,
        rect()
            .vertical()
            .expanded()
            .spacing(6.)
            .child(list_filter_bar(
                "Type to filter processes…",
                filter,
                list_filter,
                palette,
            ))
            .child(
                ScrollView::new()
                    .expanded()
                    .spacing(4.)
                    .child(list_header(
                        palette,
                        &[
                            ("Process", 30.),
                            ("PID", 8.),
                            ("Sockets", 10.),
                            ("Live", 14.),
                            ("Receive", 12.),
                            ("Send", 12.),
                            ("Total", 14.),
                        ],
                    ))
                    .children(if rows.is_empty() {
                        vec![empty_state("No matching processes.".into(), palette)]
                    } else {
                        rows
                    }),
            ),
    );

    if filter_keys {
        attach_list_filter_keys(body, list_filter, palette)
    } else {
        body
    }
}

fn connections_detail_view(
    connections: Vec<ConnectionDetail>,
    selected: ConnectionDetail,
    live_rates: Vec<LiveConnectionRate>,
    rate_tracker: RateTracker,
    selected_connection: State<Option<ConnectionDetail>>,
    palette: Palette,
    sample_tick: u64,
) -> Element {
    let fresh = connections
        .iter()
        .find(|c| c.id == selected.id)
        .cloned()
        .unwrap_or(selected);
    connection_detail_screen(
        fresh,
        &live_rates,
        rate_tracker,
        selected_connection,
        palette,
        sample_tick,
    )
}

fn connections_list_view(
    connections: Vec<ConnectionDetail>,
    live_rates: Vec<LiveConnectionRate>,
    filter: String,
    list_filter: State<String>,
    palette: Palette,
    selected_connection: State<Option<ConnectionDetail>>,
    filter_keys: bool,
) -> Element {
    let mut sorted = connections;
    sorted.sort_by(|a, b| {
        b.combined_bytes()
            .partial_cmp(&a.combined_bytes())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let needle = filter.to_ascii_lowercase();
    let rows: Vec<Element> = sorted
        .into_iter()
        .filter(|c| c.matches_filter(&needle))
        .enumerate()
        .map(|(i, conn)| {
            connection_row(
                conn,
                &live_rates,
                palette,
                selected_connection,
                i,
            )
        })
        .collect();

    let body = rect()
        .vertical()
        .expanded()
        .spacing(0.)
        .child(panel_shell(
            palette,
            rect()
                .vertical()
                .expanded()
                .spacing(6.)
                .child(list_filter_bar(
                    "Type to filter connections…",
                    filter,
                    list_filter,
                    palette,
                ))
                .child(
                    ScrollView::new()
                        .expanded()
                        .spacing(4.)
                        .child(list_header(
                            palette,
                            &[
                                ("Process", 16.),
                                ("Remote", 18.),
                                ("Local", 16.),
                                ("Proto", 6.),
                                ("Role", 6.),
                                ("State", 8.),
                                ("Live", 12.),
                                ("Total", 12.),
                            ],
                        ))
                        .children(if rows.is_empty() {
                            vec![empty_state("No matching connections.".into(), palette)]
                        } else {
                            rows
                        }),
                ),
        ));

    if filter_keys {
        attach_list_filter_keys(body.into(), list_filter, palette)
    } else {
        body.into()
    }
}

fn attach_list_filter_keys(
    body: Element,
    list_filter: State<String>,
    palette: Palette,
) -> Element {
    rect()
        .vertical()
        .expanded()
        .background(palette.bg)
        .on_global_key_down(move |e: Event<KeyboardEventData>| {
            use keyboard_types::{Key, NamedKey};
            let data = e.data();
            match &data.key {
                Key::Character(ch) if !data.modifiers.alt() && !data.modifiers.ctrl() => {
                    let mut next = list_filter.peek().clone();
                    next.push_str(ch);
                    *list_filter.write_unchecked() = next;
                }
                Key::Named(NamedKey::Backspace) => {
                    let mut next = list_filter.peek().clone();
                    next.pop();
                    *list_filter.write_unchecked() = next;
                }
                Key::Named(NamedKey::Escape) => {
                    *list_filter.write_unchecked() = String::new();
                }
                _ => {}
            }
        })
        .child(body)
        .into()
}

fn list_filter_bar(
    placeholder: &'static str,
    filter: String,
    _list_filter: State<String>,
    palette: Palette,
) -> Element {
    let hint = if filter.is_empty() {
        placeholder.to_string()
    } else {
        format!("{filter}  ·  Esc clears")
    };

    rect()
        .width(Size::fill())
        .padding(Gaps::new_all(8.))
        .background(palette.bg)
        .corner_radius(8.)
        .border(palette.border())
        .child(
            label()
                .text(hint)
                .font_size(12.)
                .color(if filter.is_empty() {
                    palette.muted
                } else {
                    palette.text
                }),
        )
        .into()
}

fn settings_view(palette: Palette) -> Element {
    ScrollView::new()
        .expanded()
        .child(settings_panel(palette))
        .into()
}

fn settings_panel(palette: Palette) -> Element {
    rect()
        .vertical()
        .width(Size::fill())
        .background(palette.panel)
        .corner_radius(12.)
        .border(palette.border())
        .padding(Gaps::new_all(12.))
        .spacing(14.)
        .child(
            rect()
                .vertical()
                .spacing(8.)
                .padding(Gaps::new(8., 0., 8., 0.))
                .child(
                    label()
                        .text("About")
                        .font_size(14.)
                        .font_weight(FontWeight::BOLD)
                        .color(palette.title),
                )
                .child(about_content(palette)),
        )
        .child(settings_row(
            "Sampling",
            "Network stats refresh every second via sysinfo and nettop.",
            palette,
        ))
        .child(settings_row(
            "Platform",
            "Process and connection views require macOS nettop/lsof.",
            palette,
        ))
        .into()
}

fn panel_shell(palette: Palette, body: impl IntoElement) -> Element {
    rect()
        .vertical()
        .expanded()
        .background(palette.panel)
        .corner_radius(12.)
        .border(palette.border())
        .padding(Gaps::new_all(12.))
        .child(body)
        .into()
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

fn empty_state(message: String, palette: Palette) -> Element {
    rect()
        .padding(Gaps::new_all(24.))
        .child(
            label()
                .text(message)
                .font_size(13.)
                .color(palette.muted),
        )
        .into()
}

fn process_row(
    proc: ProcessTraffic,
    live_rates: &[LiveConnectionRate],
    palette: Palette,
    index: usize,
) -> Element {
    let total = proc.combined_bytes();
    let (live_rx, live_tx) = rates_for_process(live_rates, &proc.name);
    let live_total = live_rx + live_tx;
    data_row_shell(palette, index, false, vec![
            label()
                .text(proc.name)
                .font_size(12.)
                .font_weight(FontWeight::BOLD)
                .color(palette.text)
                .width(Size::percent(30.))
                .into(),
            label()
                .text(proc.pid.to_string())
                .font_size(12.)
                .color(palette.muted)
                .width(Size::percent(8.))
                .into(),
            label()
                .text(proc.connection_count.to_string())
                .font_size(12.)
                .color(palette.muted)
                .width(Size::percent(10.))
                .into(),
            label()
                .text(format_rate(live_total))
                .font_size(12.)
                .font_weight(FontWeight::BOLD)
                .color(palette.total)
                .width(Size::percent(14.))
                .into(),
            label()
                .text(format_total(proc.rx_bytes))
                .font_size(12.)
                .color(palette.receive)
                .width(Size::percent(12.))
                .into(),
            label()
                .text(format_total(proc.tx_bytes))
                .font_size(12.)
                .color(palette.send)
                .width(Size::percent(12.))
                .into(),
            label()
                .text(format_total(total))
                .font_size(12.)
                .font_weight(FontWeight::BOLD)
                .color(palette.total)
                .width(Size::percent(14.))
                .into(),
        ],
    )
}

fn connection_row(
    conn: ConnectionDetail,
    live_rates: &[LiveConnectionRate],
    palette: Palette,
    mut selected_connection: State<Option<ConnectionDetail>>,
    index: usize,
) -> Element {
    let process_name = conn.process_name.clone();
    let remote = conn.remote_label();
    let local = conn.local_label();
    let transport = conn.transport.clone();
    let role = conn.role_label();
    let state = conn.state.clone();
    let total = conn.combined_bytes();
    let live = rate_for_connection(live_rates, conn.id)
        .map(|r| r.combined_bps())
        .unwrap_or(0.0);
    let conn_for_select = conn.clone();
    let is_selected = selected_connection
        .peek()
        .as_ref()
        .is_some_and(|selected| selected.id == conn.id);

    clickable_data_row_shell(
        palette,
        index,
        is_selected,
        move |e: Event<MouseEventData>| {
            e.stop_propagation();
            selected_connection.set(Some(conn_for_select.clone()));
        },
        vec![
            label()
                .text(process_name)
                .font_size(12.)
                .font_weight(FontWeight::BOLD)
                .color(palette.text)
                .width(Size::percent(16.))
                .into(),
            label()
                .text(remote)
                .font_size(11.)
                .font_weight(FontWeight::BOLD)
                .color(palette.text)
                .width(Size::percent(18.))
                .into(),
            label()
                .text(local)
                .font_size(11.)
                .color(palette.muted)
                .width(Size::percent(16.))
                .into(),
            label()
                .text(transport.to_uppercase())
                .font_size(11.)
                .color(palette.muted)
                .width(Size::percent(6.))
                .into(),
            label()
                .text(role)
                .font_size(11.)
                .color(palette.muted)
                .width(Size::percent(6.))
                .into(),
            label()
                .text(state)
                .font_size(11.)
                .color(palette.muted)
                .width(Size::percent(8.))
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
        ],
    )
}

fn data_row_shell(
    palette: Palette,
    _index: usize,
    selected: bool,
    children: Vec<Element>,
) -> Element {
    let bg = if selected {
        Color::from_argb(40, palette.receive.r(), palette.receive.g(), palette.receive.b())
    } else {
        palette.panel
    };

    rect()
        .horizontal()
        .width(Size::fill())
        .padding(Gaps::new(8., 10., 8., 10.))
        .background(bg)
        .spacing(8.)
        .children(children)
        .into()
}

fn clickable_data_row_shell(
    palette: Palette,
    index: usize,
    selected: bool,
    on_click: impl FnMut(Event<MouseEventData>) + 'static,
    children: Vec<Element>,
) -> Element {
    let bg = if selected {
        Color::from_argb(40, palette.receive.r(), palette.receive.g(), palette.receive.b())
    } else {
        palette.panel
    };

    rect()
        .horizontal()
        .width(Size::fill())
        .padding(Gaps::new(8., 10., 8., 10.))
        .background(bg)
        .spacing(8.)
        .children(children)
        .on_mouse_up(on_click)
        .into()
}

fn settings_row(title: &str, detail: &str, palette: Palette) -> Element {
    rect()
        .vertical()
        .spacing(4.)
        .child(
            label()
                .text(title.to_string())
                .font_size(13.)
                .font_weight(FontWeight::BOLD)
                .color(palette.text),
        )
        .child(
            label()
                .text(detail.to_string())
                .font_size(11.)
                .color(palette.muted),
        )
        .into()
}

fn sidebar(
    snapshot: &NetworkSnapshot,
    palette: Palette,
    app_section: State<AppSection>,
    selected: State<Option<String>>,
    selected_connection: State<Option<ConnectionDetail>>,
    started: Instant,
    active: AppSection,
    alerts: &AlertEngine,
) -> Element {
    let uptime = started.elapsed();
    let uptime_label = format!(
        "{}h {}m",
        uptime.as_secs() / 3600,
        (uptime.as_secs() % 3600) / 60
    );

    rect()
        .width(Size::px(220.))
        .height(Size::fill())
        .background(palette.panel)
        .border(
            Border::new()
                .fill(palette.panel_edge)
                .width(BorderWidth {
                    top: 0.,
                    right: 1.,
                    bottom: 0.,
                    left: 0.,
                }),
        )
        .padding(Gaps::new_all(18.))
        .vertical()
        .spacing(16.)
        .child(
            rect()
                .vertical()
                .spacing(4.)
                .child(
                    rect()
                        .horizontal()
                        .spacing(8.)
                        .child(
                            rect()
                                .width(Size::px(28.))
                                .height(Size::px(28.))
                                .background(palette.accent)
                                .corner_radius(14.),
                        )
                        .child(
                            rect()
                                .vertical()
                                .spacing(2.)
                                .child(
                                    label()
                                        .text("Osman")
                                        .font_size(22.)
                                        .font_weight(FontWeight::BOLD)
                                        .color(palette.title),
                                )
                                .child(
                                    label()
                                        .text("Network Monitor")
                                        .font_size(12.)
                                        .color(palette.muted),
                                ),
                        ),
                ),
        )
        .child(
            rect()
                .vertical()
                .spacing(10.)
                .padding(Gaps::new_all(12.))
                .background(palette.bg)
                .corner_radius(12.)
                .border(palette.border())
                .child(
                    label()
                        .text("Total (all adapters)")
                        .font_size(10.)
                        .font_weight(FontWeight::BOLD)
                        .color(palette.muted),
                )
                .child(sidebar_stat(ProcessLane::Red, snapshot.total_rx_bps, palette))
                .child(sidebar_stat(ProcessLane::Blue, snapshot.total_tx_bps, palette))
                .child(sidebar_stat(
                    ProcessLane::Green,
                    snapshot.total_rx_bps + snapshot.total_tx_bps,
                    palette,
                )),
        )
        .child(
            rect()
                .vertical()
                .spacing(6.)
                .child(
                    label()
                        .text("System")
                        .font_size(10.)
                        .font_weight(FontWeight::BOLD)
                        .color(palette.muted),
                )
                .child(sidebar_meta(
                    "Adapters",
                    snapshot.interfaces.len().to_string(),
                    palette,
                ))
                .child(sidebar_meta(
                    "Processes",
                    snapshot.process_count.to_string(),
                    palette,
                ))
                .child(sidebar_meta(
                    "Connections",
                    snapshot.connection_count.to_string(),
                    palette,
                ))
                .child(sidebar_meta("Uptime", uptime_label, palette)),
        )
        .child(
            rect()
                .vertical()
                .spacing(4.)
                .child(nav_item(
                    AppSection::Overview,
                    active,
                    app_section,
                    selected,
                    selected_connection,
                    palette,
                ))
                .child(nav_item(
                    AppSection::Adapters,
                    active,
                    app_section,
                    selected,
                    selected_connection,
                    palette,
                ))
                .child(nav_item(
                    AppSection::Processes,
                    active,
                    app_section,
                    selected,
                    selected_connection,
                    palette,
                ))
                .child(nav_item(
                    AppSection::Connections,
                    active,
                    app_section,
                    selected,
                    selected_connection,
                    palette,
                ))
                .child(nav_item(
                    AppSection::TrafficCharacter,
                    active,
                    app_section,
                    selected,
                    selected_connection,
                    palette,
                ))
                .child(nav_item(
                    AppSection::Alerts,
                    active,
                    app_section,
                    selected,
                    selected_connection,
                    palette,
                ))
                .child(nav_item(
                    AppSection::Settings,
                    active,
                    app_section,
                    selected,
                    selected_connection,
                    palette,
                )),
        )
        .child(rect().height(Size::fill()))
        .child(sidebar_status_footer(palette, alerts))
        .into()
}

fn sidebar_status_footer(palette: Palette, alerts: &AlertEngine) -> Element {
    let recent = alerts.events().back();
    let (status, color) = if let Some(event) = recent {
        let age = event.at.elapsed();
        if age < Duration::from_secs(120) {
            (event.message.clone(), palette.receive)
        } else {
            ("Live · 1 Hz sampling".into(), palette.send)
        }
    } else {
        ("Live · 1 Hz sampling".into(), palette.send)
    };

    rect()
        .vertical()
        .spacing(6.)
        .child(
            rect()
                .horizontal()
                .spacing(6.)
                .child(
                    rect()
                        .width(Size::px(7.))
                        .height(Size::px(7.))
                        .background(color)
                        .corner_radius(4.),
                )
                .child(
                    label()
                        .text(status)
                        .font_size(10.)
                        .color(color),
                ),
        )
        .child(
            label()
                .text(format!("v{}", env!("CARGO_PKG_VERSION")))
                .font_size(10.)
                .color(palette.muted),
        )
        .into()
}

fn nav_item(
    section: AppSection,
    active: AppSection,
    mut app_section: State<AppSection>,
    selected: State<Option<String>>,
    mut selected_connection: State<Option<ConnectionDetail>>,
    palette: Palette,
) -> Element {
    let nav_label = match section {
        AppSection::Overview => "Overview",
        AppSection::Adapters => "Adapters",
        AppSection::Processes => "Processes",
        AppSection::Connections => "Connections",
        AppSection::TrafficCharacter => "Traffic Character",
        AppSection::Alerts => "Alerts",
        AppSection::Settings => "Settings",
    };
    let is_active = active == section;
    let bg = if is_active {
        Color::from_argb(40, palette.receive.r(), palette.receive.g(), palette.receive.b())
    } else {
        palette.panel
    };
    let text_color = if is_active {
        palette.receive
    } else {
        palette.muted
    };
    let section_set = section;

    rect()
        .width(Size::fill())
        .padding(Gaps::new_all(8.))
        .background(bg)
        .corner_radius(8.)
        .on_mouse_up(move |_| {
            app_section.set(section_set);
            if !matches!(section_set, AppSection::Overview | AppSection::Adapters) {
                *selected.write_unchecked() = None;
            }
            if section_set != AppSection::Connections {
                selected_connection.set(None);
            }
        })
        .child(
            label()
                .text(nav_label)
                .font_size(12.)
                .font_weight(if is_active {
                    FontWeight::BOLD
                } else {
                    FontWeight::NORMAL
                })
                .color(if is_active {
                    palette.text
                } else {
                    text_color
                }),
        )
        .into()
}

fn sidebar_stat(lane: ProcessLane, rate: f64, palette: Palette) -> Element {
    rect()
        .horizontal()
        .width(Size::fill())
        .spacing(8.)
        .child(lane_dot(lane, palette))
        .child(
            rect()
                .vertical()
                .spacing(1.)
                .child(
                    label()
                        .text(lane.label())
                        .font_size(10.)
                        .color(palette.muted),
                )
                .child(
                    label()
                        .text(format_rate(rate))
                        .font_size(15.)
                        .font_weight(FontWeight::BOLD)
                        .color(lane.color(palette)),
                ),
        )
        .into()
}

fn sidebar_meta(label_text: &'static str, value: String, palette: Palette) -> Element {
    rect()
        .horizontal()
        .width(Size::fill())
        .child(
            label()
                .text(label_text)
                .font_size(11.)
                .color(palette.muted),
        )
        .child(
            label()
                .text(value)
                .font_size(11.)
                .font_weight(FontWeight::BOLD)
                .color(palette.text),
        )
        .into()
}

fn detail_panel(
    adapter_name: String,
    snapshot: &NetworkSnapshot,
    detail: Option<InterfaceDetail>,
    live_rates: Vec<LiveConnectionRate>,
    palette: Palette,
    window: TimeWindow,
    chart_scales: State<ChartScaleBank>,
) -> Element {
    let mut chart_scales = chart_scales;
    let stats = snapshot
        .interfaces
        .iter()
        .find(|i| i.name == adapter_name)
        .cloned();
    let iface_rates = rates_for_interface(&live_rates, &adapter_name);
    let detail_max_y = stats.as_ref().map(|s| {
        let rx = slice_history(&s.rx_history, window);
        let tx = slice_history(&s.tx_history, window);
        let combined = slice_history(&s.combined_history, window);
        let key = scope_id(&adapter_name, ProcessLane::Green);
        chart_scales
            .write()
            .adapter_y(key.wrapping_add(1), window, &rx, &tx, &combined)
    });

    rect()
        .vertical()
        .width(Size::fill())
        .height(Size::px(280.))
        .background(palette.panel)
        .corner_radius(12.)
        .border(palette.border())
        .padding(Gaps::new_all(12.))
        .spacing(10.)
        .child(
            rect()
                .horizontal()
                .width(Size::fill())
                .height(Size::flex(1.))
                .spacing(10.)
                .child(detail_col_details(detail.as_ref(), palette))
                .child(detail_col_activity(
                    stats.as_ref(),
                    palette,
                    window,
                    detail_max_y,
                    snapshot.sample_tick,
                ))
                .child(detail_col_statistics(detail.as_ref(), palette))
                .child(detail_col_talkers(detail.as_ref(), &iface_rates, palette)),
        )
        .into()
}

fn detail_col_activity(
    stats: Option<&InterfaceStats>,
    palette: Palette,
    window: TimeWindow,
    max_y: Option<f64>,
    sample_tick: u64,
) -> Element {
    let combined = stats
        .map(|s| slice_history(&s.combined_history, window))
        .unwrap_or_default();
    let activity_key = sample_tick;
    let rx = stats
        .map(|s| slice_history(&s.rx_history, window))
        .unwrap_or_default();
    let tx = stats
        .map(|s| slice_history(&s.tx_history, window))
        .unwrap_or_default();
    let scale = max_y.unwrap_or(MIN_CHART_SCALE);

    rect()
        .vertical()
        .width(Size::percent(25.))
        .spacing(6.)
        .padding(Gaps::new_all(10.))
        .background(palette.bg)
        .corner_radius(10.)
        .border(palette.border())
        .child(
            label()
                .text(format!("Activity ({})", window.subtitle()))
                .font_size(10.)
                .font_weight(FontWeight::BOLD)
                .color(palette.muted),
        )
        .child(
            canvas(RenderCallback::new(move |ctx| {
                draw_network_activity(ctx, &rx, &tx, &combined, palette, window, scale);
            }))
            .width(Size::fill())
            .height(Size::px(200.))
            .key(activity_key),
        )
        .into()
}

fn detail_col_details(detail: Option<&InterfaceDetail>, palette: Palette) -> Element {
    let mut card = rect()
        .vertical()
        .width(Size::percent(25.))
        .spacing(4.)
        .padding(Gaps::new_all(10.))
        .background(palette.bg)
        .corner_radius(10.)
        .border(palette.border())
        .child(
            label()
                .text("Details")
                .font_size(10.)
                .font_weight(FontWeight::BOLD)
                .color(palette.muted),
        );

    card = match detail {
        Some(d) => {
            let status_color = if d.status_label() == "Connected" {
                palette.send
            } else {
                palette.muted
            };
            card.child(detail_kv_row(
                "Status".into(),
                d.status_label().into(),
                palette,
                Some(status_color),
            ))
            .child(detail_kv_row("IPv4 Address".into(), d.ipv4(), palette, None))
            .child(detail_kv_row("IPv6 Address".into(), d.ipv6(), palette, None))
            .child(detail_kv_row("MAC Address".into(), d.mac.clone(), palette, None))
            .child(detail_kv_row("MTU".into(), d.mtu.to_string(), palette, None))
        }
        None => card.child(detail_loading(palette)),
    };

    card.into()
}

fn detail_col_statistics(detail: Option<&InterfaceDetail>, palette: Palette) -> Element {
    let mut card = rect()
        .vertical()
        .width(Size::percent(25.))
        .spacing(4.)
        .padding(Gaps::new_all(10.))
        .background(palette.bg)
        .corner_radius(10.)
        .border(palette.border())
        .child(
            label()
                .text("Statistics")
                .font_size(10.)
                .font_weight(FontWeight::BOLD)
                .color(palette.muted),
        );

    card = match detail {
        Some(d) => {
            let rx = d
                .stats
                .as_ref()
                .map(|s| format_total(s.total_rx))
                .unwrap_or_else(|| "—".into());
            let tx = d
                .stats
                .as_ref()
                .map(|s| format_total(s.total_tx))
                .unwrap_or_else(|| "—".into());
            let drops = if d.drops == 0 {
                "—".to_string()
            } else {
                d.drops.to_string()
            };
            card.child(detail_kv_row("Bytes Received".into(), rx, palette, None))
                .child(detail_kv_row("Bytes Sent".into(), tx, palette, None))
                .child(detail_kv_row(
                    "Packets Received".into(),
                    d.packets_in.to_string(),
                    palette,
                    None,
                ))
                .child(detail_kv_row(
                    "Packets Sent".into(),
                    d.packets_out.to_string(),
                    palette,
                    None,
                ))
                .child(detail_kv_row(
                    "Errors".into(),
                    d.errors.to_string(),
                    palette,
                    None,
                ))
                .child(detail_kv_row("Drops".into(), drops, palette, None))
        }
        None => card.child(detail_loading(palette)),
    };

    card.into()
}

fn detail_col_talkers(
    detail: Option<&InterfaceDetail>,
    live_rates: &[LiveConnectionRate],
    palette: Palette,
) -> Element {
    let mut card = rect()
        .vertical()
        .width(Size::percent(25.))
        .spacing(6.)
        .padding(Gaps::new_all(10.))
        .background(palette.bg)
        .corner_radius(10.)
        .border(palette.border())
        .child(
            label()
                .text("Top Talkers (by Total)")
                .font_size(10.)
                .font_weight(FontWeight::BOLD)
                .color(palette.muted),
        )
        .children(top_talker_rows(detail, live_rates, palette));

    if let Some(d) = detail {
        if !d.connections.is_empty() {
            card = card.child(
                label()
                    .text(format!("Sockets ({})", d.connections.len()))
                    .font_size(10.)
                    .font_weight(FontWeight::BOLD)
                    .color(palette.muted),
            );
            for conn in d.connections.iter().take(6) {
                let live = rate_for_connection(live_rates, conn.id)
                    .map(|r| format_rate(r.combined_bps()))
                    .unwrap_or_else(|| "—".into());
                card = card.child(
                    rect()
                        .horizontal()
                        .width(Size::fill())
                        .spacing(4.)
                        .child(
                            label()
                                .text(conn.remote_label())
                                .font_size(9.)
                                .color(palette.text)
                                .width(Size::flex(1.)),
                        )
                        .child(
                            label()
                                .text(live)
                                .font_size(9.)
                                .color(palette.muted),
                        ),
                );
            }
        }
    }

    card.into()
}

fn detail_kv_row(
    label_text: String,
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
                .font_size(10.)
                .color(palette.muted)
                .width(Size::percent(46.)),
        )
        .child(
            label()
                .text(value)
                .font_size(10.)
                .font_weight(FontWeight::BOLD)
                .color(value_color.unwrap_or(palette.text))
                .width(Size::flex(1.)),
        )
        .into()
}

fn detail_loading(palette: Palette) -> Element {
    label()
        .text("Loading…")
        .font_size(10.)
        .color(palette.muted)
        .into()
}

fn top_talker_rows(
    detail: Option<&InterfaceDetail>,
    live_rates: &[LiveConnectionRate],
    palette: Palette,
) -> Vec<Element> {
    let Some(_detail) = detail else {
        return vec![detail_loading(palette)];
    };

    let mut ranked: Vec<_> = live_rates.iter().collect();
    ranked.sort_by(|a, b| {
        b.combined_bps()
            .partial_cmp(&a.combined_bps())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    ranked.truncate(4);

    if ranked.is_empty() {
        return vec![label()
            .text("No connections yet")
            .font_size(10.)
            .color(palette.muted)
            .into()];
    }

    let max = ranked
        .iter()
        .map(|c| c.combined_bps())
        .fold(1.0_f64, f64::max)
        .max(1.0);

    ranked
        .into_iter()
        .map(|conn| {
            let total_bps = conn.combined_bps();
            let pct = ((total_bps / max) * 100.0).clamp(8.0, 100.0) as f32;
            let host = conn.remote_label.clone();
            let rate_label = if total_bps > 0.0 {
                format_rate(total_bps)
            } else {
                "—".into()
            };

            rect()
                .horizontal()
                .width(Size::fill())
                .spacing(6.)
                .child(
                    rect()
                        .vertical()
                        .spacing(3.)
                        .width(Size::flex(1.))
                        .child(
                            label()
                                .text(host)
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
                                        .width(Size::percent(pct))
                                        .height(Size::px(5.))
                                        .background(palette.total)
                                        .corner_radius(3.),
                                ),
                        ),
                )
                .child(
                    label()
                        .text(rate_label)
                        .font_size(10.)
                        .font_weight(FontWeight::BOLD)
                        .color(palette.text),
                )
                .into()
        })
        .collect()
}

fn lane_dot(lane: ProcessLane, palette: Palette) -> Element {
    rect()
        .width(Size::px(8.))
        .height(Size::px(8.))
        .background(lane.color(palette))
        .corner_radius(4.)
        .into()
}

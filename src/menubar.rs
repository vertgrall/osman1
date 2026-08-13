use std::sync::Mutex;

use freya::prelude::*;
use freya::tray::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent, TrayEvent,
};
use freya::winit::window::WindowId;

use crate::about::{about_window, ABOUT_WINDOW_H, ABOUT_WINDOW_W};
use crate::adapters::adapter_title;
use crate::icon_assets;
use crate::network::{InterfaceStats, NetworkSnapshot};
use crate::preferences;
use crate::theme::{format_rate, Palette, ProcessLane};
use crate::time_window::{slice_history, TimeWindow};

const MENU_SHOW: &str = "osman.show";
const MENU_ABOUT: &str = "osman.about";
const MENU_QUIT: &str = "osman.quit";

static LATEST: Mutex<Option<NetworkSnapshot>> = Mutex::new(None);
static MINI_TARGET_INDEX: Mutex<usize> = Mutex::new(0);

type RendererDispatch = Box<dyn Fn(Box<dyn FnOnce(&mut RendererContext)>) + Send + Sync>;
static RENDERER_DISPATCH: Mutex<Option<RendererDispatch>> = Mutex::new(None);

/// Register a path to the winit renderer from launch tasks (outside component scope).
pub fn set_renderer_dispatch(dispatch: RendererDispatch) {
    let mut slot = RENDERER_DISPATCH.lock().expect("renderer dispatch lock");
    if slot.is_none() {
        *slot = Some(dispatch);
    }
}

fn post_to_renderer(f: impl FnOnce(&mut RendererContext) + 'static) {
    if let Ok(guard) = RENDERER_DISPATCH.lock() {
        if let Some(dispatch) = guard.as_ref() {
            dispatch(Box::new(f));
        }
    }
}

#[cfg(test)]
pub fn set_renderer_dispatch_for_test(dispatch: RendererDispatch) {
    *RENDERER_DISPATCH.lock().expect("renderer dispatch lock") = Some(dispatch);
}

std::thread_local! {
    static TRAY: std::cell::RefCell<Option<TrayIcon>> = const { std::cell::RefCell::new(None) };
    static POPOVER_ID: std::cell::Cell<Option<WindowId>> = const { std::cell::Cell::new(None) };
    static ABOUT_WINDOW_ID: std::cell::Cell<Option<WindowId>> = const { std::cell::Cell::new(None) };
}

/// Attach the macOS menu bar monitor to a launch config.
pub fn with_menubar(config: LaunchConfig) -> LaunchConfig {
    config.with_tray(build_tray, handle_tray_event)
}

pub fn update_from_snapshot(snapshot: &NetworkSnapshot) {
    if let Ok(mut guard) = LATEST.lock() {
        *guard = Some(snapshot.clone());
    }

    apply_tray_title(snapshot);

    let tooltip = menubar_tooltip(snapshot);
    post_to_renderer(move |_ctx| {
        TRAY.with(|cell| {
            if let Some(tray) = cell.borrow().as_ref() {
                let _ = tray.set_tooltip(Some(tooltip));
            }
        });
    });
}

fn mini_palette() -> Palette {
    preferences::ensure_init();
    preferences::get().app_theme().palette()
}

fn mini_target_index() -> usize {
    MINI_TARGET_INDEX.lock().map(|g| *g).unwrap_or(0)
}

fn set_mini_target_index(index: usize) {
    if let Ok(mut guard) = MINI_TARGET_INDEX.lock() {
        *guard = index;
    }
}

#[derive(Clone, Debug, PartialEq)]
struct MiniMonitorTarget {
    label: String,
    detail: String,
    rx_bps: f64,
    tx_bps: f64,
    rx_history: Vec<f64>,
    tx_history: Vec<f64>,
    combined_history: Vec<f64>,
}

fn ranked_interfaces<'a>(snap: &'a NetworkSnapshot) -> Vec<&'a InterfaceStats> {
    let mut ranked: Vec<&InterfaceStats> = snap.interfaces.iter().collect();
    ranked.sort_by(|a, b| {
        b.combined_bps
            .partial_cmp(&a.combined_bps)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.name.cmp(&b.name))
    });
    ranked
}

fn mini_target_count(snap: &NetworkSnapshot) -> usize {
    1 + ranked_interfaces(snap).len()
}

fn normalize_target_index(index: usize, count: usize) -> usize {
    if count == 0 {
        0
    } else {
        index % count
    }
}

fn cycle_mini_target(delta: i32, snap: &NetworkSnapshot) -> usize {
    let count = mini_target_count(snap).max(1);
    let current = normalize_target_index(mini_target_index(), count);
    let next = if delta < 0 {
        (current + count - 1) % count
    } else {
        (current + 1) % count
    };
    set_mini_target_index(next);
    next
}

fn resolve_mini_target(snap: &NetworkSnapshot, index: usize) -> MiniMonitorTarget {
    let count = mini_target_count(snap).max(1);
    let index = normalize_target_index(index, count);

    if index == 0 {
        return MiniMonitorTarget {
            label: "All Traffic".into(),
            detail: format!(
                "{} adapter{}",
                snap.interfaces.len(),
                if snap.interfaces.len() == 1 { "" } else { "s" }
            ),
            rx_bps: snap.total_rx_bps,
            tx_bps: snap.total_tx_bps,
            rx_history: snap.rx_history.clone(),
            tx_history: snap.tx_history.clone(),
            combined_history: snap.combined_history.clone(),
        };
    }

    let iface = ranked_interfaces(snap).get(index - 1).copied();

    if let Some(iface) = iface {
        MiniMonitorTarget {
            label: adapter_title(&iface.name),
            detail: iface.name.clone(),
            rx_bps: iface.rx_bps,
            tx_bps: iface.tx_bps,
            rx_history: iface.rx_history.clone(),
            tx_history: iface.tx_history.clone(),
            combined_history: iface.combined_history.clone(),
        }
    } else {
        MiniMonitorTarget {
            label: "All Traffic".into(),
            detail: "No adapters".into(),
            rx_bps: snap.total_rx_bps,
            tx_bps: snap.total_tx_bps,
            rx_history: snap.rx_history.clone(),
            tx_history: snap.tx_history.clone(),
            combined_history: snap.combined_history.clone(),
        }
    }
}

fn apply_tray_title(snapshot: &NetworkSnapshot) {
    let target = resolve_mini_target(snapshot, mini_target_index());
    let title = menubar_title(target.rx_bps, target.tx_bps, &target.label);
    post_to_renderer(move |_ctx| {
        TRAY.with(|cell| {
            if let Some(tray) = cell.borrow().as_ref() {
                tray.set_title(Some(title));
            }
        });
    });
}

fn menubar_popover() -> Element {
    preferences::ensure_init();
    let snap = LATEST
        .lock()
        .ok()
        .and_then(|g| g.clone())
        .unwrap_or_default();
    let palette = mini_palette();
    let target_index = use_state(|| mini_target_index());
    let refresh_tick = use_state(|| snap.sample_tick);

    use_future(move || {
        let mut refresh_tick = refresh_tick;
        async move {
            loop {
                async_io::Timer::after(std::time::Duration::from_millis(900)).await;
                let tick = LATEST
                    .lock()
                    .ok()
                    .and_then(|g| g.as_ref().map(|s| s.sample_tick))
                    .unwrap_or(0);
                refresh_tick.set(tick);
            }
        }
    });

    let _live = *refresh_tick.read();
    let live_snap = LATEST
        .lock()
        .ok()
        .and_then(|g| g.clone())
        .unwrap_or(snap);
    let count = mini_target_count(&live_snap).max(1);
    let index = normalize_target_index(*target_index.read(), count);
    let target = resolve_mini_target(&live_snap, index);
    let total = target.rx_bps + target.tx_bps;
    let rx_slice = slice_history(&target.rx_history, TimeWindow::Sec60);
    let tx_slice = slice_history(&target.tx_history, TimeWindow::Sec60);
    let combined_slice = slice_history(&target.combined_history, TimeWindow::Sec60);
    let can_cycle = count > 1;
    let position_label = format!("{} of {}", index + 1, count);

    rect()
        .vertical()
        .width(Size::fill())
        .height(Size::fill())
        .background(palette.bg)
        .child(
            rect()
                .vertical()
                .width(Size::fill())
                .padding(Gaps::new_all(12.))
                .spacing(8.)
                .background(palette.panel)
                .corner_radius(12.)
                .border(palette.elevated_border())
                .child(
                    rect()
                        .horizontal()
                        .width(Size::fill())
                        .height(Size::px(3.))
                        .background(palette.accent)
                        .corner_radius(2.),
                )
                .child(
                    label()
                        .text("Osman — Live")
                        .font_size(13.)
                        .font_weight(FontWeight::BOLD)
                        .color(palette.text),
                )
                .child(mini_target_header(
                    &target.label,
                    can_cycle,
                    palette,
                    target_index,
                    live_snap.clone(),
                ))
                .child(
                    rect()
                        .horizontal()
                        .spacing(12.)
                        .child(mini_stat(ProcessLane::Red, target.rx_bps, palette))
                        .child(mini_stat(ProcessLane::Blue, target.tx_bps, palette))
                        .child(mini_stat(ProcessLane::Green, total, palette)),
                )
                .child(
                    canvas(RenderCallback::new(move |ctx| {
                        let max_y =
                            crate::charts::chart_peak_max(&rx_slice, &tx_slice, &combined_slice);
                        crate::charts::draw_activity_sparkline(
                            ctx,
                            &rx_slice,
                            &tx_slice,
                            &combined_slice,
                            palette,
                            max_y,
                        );
                    }))
                    .width(Size::fill())
                    .height(Size::px(72.)),
                )
                .child(
                    label()
                        .text(format!("{} · {}", position_label, target.detail))
                        .font_size(10.)
                        .color(palette.muted),
                ),
        )
        .into()
}

fn mini_target_header(
    target_label: &str,
    can_cycle: bool,
    palette: Palette,
    mut target_index: State<usize>,
    snap: NetworkSnapshot,
) -> Element {
    let title = target_label.to_string();
    rect()
        .horizontal()
        .width(Size::fill())
        .spacing(6.)
        .child(mini_cycle_button(
            "‹",
            can_cycle,
            palette,
            {
                let snap = snap.clone();
                move |_| {
                    let next = cycle_mini_target(-1, &snap);
                    target_index.set(next);
                    apply_tray_title(&snap);
                }
            },
        ))
        .child(
            rect()
                .expanded()
                .child(
                    label()
                        .text(title)
                        .font_size(12.)
                        .font_weight(FontWeight::BOLD)
                        .color(palette.title)
                        .width(Size::fill()),
                ),
        )
        .child(mini_cycle_button(
            "›",
            can_cycle,
            palette,
            {
                let snap = snap.clone();
                move |_| {
                    let next = cycle_mini_target(1, &snap);
                    target_index.set(next);
                    apply_tray_title(&snap);
                }
            },
        ))
        .into()
}

fn mini_cycle_button(
    chevron: &str,
    enabled: bool,
    palette: Palette,
    on_press: impl FnMut(Event<MouseEventData>) + 'static,
) -> Element {
    let text_color = if enabled {
        palette.text
    } else {
        palette.muted
    };
    let bg = if enabled {
        Color::from_argb(28, palette.accent.r(), palette.accent.g(), palette.accent.b())
    } else {
        Color::from_argb(8, palette.text.r(), palette.text.g(), palette.text.b())
    };

    let mut button = rect()
        .width(Size::px(28.))
        .height(Size::px(28.))
        .background(bg)
        .corner_radius(8.)
        .border(palette.border())
        .child(
            label()
                .text(chevron.to_string())
                .font_size(16.)
                .font_weight(FontWeight::BOLD)
                .color(text_color),
        );

    if enabled {
        button = button.on_mouse_up(on_press);
    }

    button.into()
}

fn mini_stat(lane: ProcessLane, rate: f64, palette: Palette) -> Element {
    rect()
        .vertical()
        .spacing(2.)
        .child(
            label()
                .text(lane.label())
                .font_size(9.)
                .color(palette.muted),
        )
        .child(
            label()
                .text(format_rate(rate))
                .font_size(12.)
                .font_weight(FontWeight::BOLD)
                .color(lane.color(palette)),
        )
        .into()
}

fn build_tray() -> TrayIcon {
    let show = MenuItem::with_id(MENU_SHOW, "Show Osman", true, None);
    let about = MenuItem::with_id(MENU_ABOUT, "About Osman", true, None);
    let quit = MenuItem::with_id(MENU_QUIT, "Quit Osman", true, None);
    let separator = PredefinedMenuItem::separator();
    let menu = Menu::with_items(&[&show, &about, &separator, &quit]).expect("menubar menu");

    let tray = TrayIconBuilder::new()
        .with_icon(crate::icon_assets::menubar_icon())
        .with_title("Osman")
        .with_tooltip("Osman by NT — network monitor")
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(false)
        .build()
        .expect("menubar tray");

    TRAY.with(|cell| *cell.borrow_mut() = Some(tray.clone()));
    tray
}

fn handle_tray_event(event: TrayEvent, mut ctx: RendererContext) {
    match event {
        TrayEvent::Icon(TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        }) => toggle_popover(&mut ctx),
        TrayEvent::Menu(menu_event) if menu_event.id == MENU_SHOW => {
            focus_main_window(&mut ctx);
        }
        TrayEvent::Menu(menu_event) if menu_event.id == MENU_ABOUT => {
            request_about_window();
        }
        TrayEvent::Menu(menu_event) if menu_event.id == MENU_QUIT => ctx.exit(),
        _ => {}
    }
}

fn toggle_popover(ctx: &mut RendererContext) {
    if let Some(id) = POPOVER_ID.with(|c| c.get()) {
        if let Some(app) = ctx.windows.get_mut(&id) {
            let visible = app.window().is_visible().unwrap_or(false);
            app.window().set_visible(!visible);
            if !visible {
                app.window().focus_window();
            }
            return;
        }
    }

    let existing = ctx.windows.iter().find_map(|(id, app)| {
        let title = app.window().title();
        if title.contains("Osman Mini") {
            Some(*id)
        } else {
            None
        }
    });

    if let Some(id) = existing {
        POPOVER_ID.with(|c| c.set(Some(id)));
        if let Some(app) = ctx.windows.get_mut(&id) {
            app.window().set_visible(true);
            app.window().focus_window();
        }
        return;
    }

    let id = ctx.launch_window(
        WindowConfig::new(menubar_popover)
            .with_title("Osman Mini")
            .with_size(360., 210.)
            .with_decorations(false)
            .with_background(mini_palette().panel),
    );
    POPOVER_ID.with(|c| c.set(Some(id)));
}

/// Open (or focus) the small About window — safe from tray, App menu, Settings.
pub fn request_about_window() {
    post_to_renderer(launch_about_window);
}

fn launch_about_window(ctx: &mut RendererContext) {
    crate::about_assets::preload();

    if let Some(id) = ABOUT_WINDOW_ID.get() {
        if let Some(app) = ctx.windows.get_mut(&id) {
            app.window().set_visible(true);
            app.window().focus_window();
            return;
        }
        ABOUT_WINDOW_ID.set(None);
    }

    preferences::ensure_init();
    let palette = preferences::get().app_theme().palette();
    let id = ctx.launch_window(
        WindowConfig::new(about_window)
            .with_title("About Osman")
            .with_size(ABOUT_WINDOW_W as f64, ABOUT_WINDOW_H as f64)
            .with_max_size(ABOUT_WINDOW_W as f64, ABOUT_WINDOW_H as f64)
            .with_resizable(false)
            .with_background(palette.bg)
            .with_icon(icon_assets::window_icon())
            .with_on_close(|_ctx, closed_id| {
                ABOUT_WINDOW_ID.with(|slot| {
                    if slot.get() == Some(closed_id) {
                        slot.set(None);
                    }
                });
                CloseDecision::Close
            }),
    );
    ABOUT_WINDOW_ID.set(Some(id));

    if let Some(app) = ctx.windows.get_mut(&id) {
        app.window().set_visible(true);
        app.window().focus_window();
    }
}

fn focus_main_window(ctx: &mut RendererContext) {
    let target = ctx.windows.iter().find_map(|(id, app)| {
        let title = app.window().title();
        if title.contains("Osman Mini") || title.contains("About Osman") {
            None
        } else {
            Some(*id)
        }
    });

    if let Some(id) = target {
        if let Some(app) = ctx.windows.get_mut(&id) {
            app.window().set_visible(true);
            app.window().focus_window();
        }
    }
}

fn menubar_title(rx_bps: f64, tx_bps: f64, _target_label: &str) -> String {
    let total = rx_bps + tx_bps;
    if total < 1.0 {
        "Osman".into()
    } else {
        format_rate(total)
    }
}

fn menubar_tooltip(snapshot: &NetworkSnapshot) -> String {
    let target = resolve_mini_target(snapshot, mini_target_index());
    let total = target.rx_bps + target.tx_bps;
    if total < 1.0 {
        format!("Osman by NT\n{}\nNo traffic", target.label)
    } else {
        format!(
            "Osman by NT\n{}\n↓ Receive {}\n↑ Send {}\nTotal {}",
            target.label,
            format_rate(target.rx_bps),
            format_rate(target.tx_bps),
            format_rate(total),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::{
        cycle_mini_target, mini_target_count, normalize_target_index, request_about_window,
        resolve_mini_target, set_mini_target_index, set_renderer_dispatch_for_test,
    };
    use crate::mock_traffic;
    use crate::network::{InterfaceStats, NetworkSnapshot};

    fn reset_target_index() {
        set_mini_target_index(0);
    }

    #[test]
    fn mini_target_cycles_system_and_adapters() {
        reset_target_index();
        let snap = mock_traffic::network_snapshot();
        let count = mini_target_count(&snap);
        assert!(count >= 2, "demo snapshot should include multiple targets");

        assert_eq!(resolve_mini_target(&snap, 0).label, "All Traffic");
        let first = cycle_mini_target(1, &snap);
        assert_eq!(first, 1);
        assert_ne!(resolve_mini_target(&snap, first).label, "All Traffic");

        set_mini_target_index(count - 1);
        let wrapped = cycle_mini_target(1, &snap);
        assert_eq!(wrapped, 0);
        reset_target_index();
    }

    #[test]
    fn mini_target_index_wraps_backward() {
        reset_target_index();
        let snap = mock_traffic::network_snapshot();
        let count = mini_target_count(&snap);
        set_mini_target_index(0);
        let prev = cycle_mini_target(-1, &snap);
        assert_eq!(prev, count - 1);
        reset_target_index();
    }

    #[test]
    fn mini_target_clamps_when_adapters_shrink() {
        reset_target_index();
        set_mini_target_index(5);
        let snap = NetworkSnapshot {
            interfaces: vec![InterfaceStats {
                name: "en0".into(),
                rx_bps: 1.0,
                tx_bps: 2.0,
                combined_bps: 3.0,
                total_rx: 0,
                total_tx: 0,
                consistency: 1.0,
                heavy_consistent: false,
                rx_history: vec![1.0],
                tx_history: vec![2.0],
                combined_history: vec![3.0],
            }],
            ..Default::default()
        };
        assert_eq!(normalize_target_index(5, mini_target_count(&snap)), 1);
        reset_target_index();
    }

    #[test]
    fn request_about_window_uses_renderer_dispatch_not_freya_context() {
        let ran = Arc::new(Mutex::new(false));
        let ran_cb = ran.clone();
        set_renderer_dispatch_for_test(Box::new(move |cb| {
            *ran_cb.lock().expect("lock") = true;
            let _ = cb;
        }));

        request_about_window();

        assert!(
            *ran.lock().expect("lock"),
            "About menu path must post through renderer dispatch (safe outside Freya component scope)"
        );
    }
}

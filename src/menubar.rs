use std::sync::Mutex;

use freya::prelude::*;
use freya::tray::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent, TrayEvent,
};
use freya::winit::window::WindowId;

use crate::network::NetworkSnapshot;
use crate::theme::{format_rate, Palette, ProcessLane};
use crate::time_window::{slice_history, TimeWindow};

const MENU_SHOW: &str = "osman.show";
const MENU_ABOUT: &str = "osman.about";
const MENU_QUIT: &str = "osman.quit";

static LATEST: Mutex<Option<NetworkSnapshot>> = Mutex::new(None);

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
fn set_renderer_dispatch_for_test(dispatch: RendererDispatch) {
    *RENDERER_DISPATCH.lock().expect("renderer dispatch lock") = Some(dispatch);
}

std::thread_local! {
    static TRAY: std::cell::RefCell<Option<TrayIcon>> = const { std::cell::RefCell::new(None) };
    static POPOVER_ID: std::cell::Cell<Option<WindowId>> = const { std::cell::Cell::new(None) };
}

/// Attach the macOS menu bar monitor to a launch config.
pub fn with_menubar(config: LaunchConfig) -> LaunchConfig {
    config.with_tray(build_tray, handle_tray_event)
}

pub fn update_from_snapshot(snapshot: &NetworkSnapshot) {
    if let Ok(mut guard) = LATEST.lock() {
        *guard = Some(snapshot.clone());
    }

    let title = menubar_title(snapshot.total_rx_bps, snapshot.total_tx_bps);
    let tooltip = menubar_tooltip(snapshot.total_rx_bps, snapshot.total_tx_bps);

    post_to_renderer(move |_ctx| {
        TRAY.with(|cell| {
            if let Some(tray) = cell.borrow().as_ref() {
                tray.set_title(Some(title));
                let _ = tray.set_tooltip(Some(tooltip));
            }
        });
    });
}

fn menubar_popover() -> Element {
    let snap = LATEST
        .lock()
        .ok()
        .and_then(|g| g.clone())
        .unwrap_or_default();
    let palette = Palette::default();
    let total = snap.total_rx_bps + snap.total_tx_bps;
    let rx_slice = slice_history(&snap.rx_history, TimeWindow::Sec60);
    let tx_slice = slice_history(&snap.tx_history, TimeWindow::Sec60);
    let combined_slice = slice_history(&snap.combined_history, TimeWindow::Sec60);

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
                .child(
                    rect()
                        .horizontal()
                        .spacing(12.)
                        .child(mini_stat(ProcessLane::Red, snap.total_rx_bps, palette))
                        .child(mini_stat(ProcessLane::Blue, snap.total_tx_bps, palette))
                        .child(mini_stat(ProcessLane::Green, total, palette)),
                )
                .child(
                    canvas(RenderCallback::new(move |ctx| {
                        let max_y = crate::charts::chart_peak_max(&rx_slice, &tx_slice, &combined_slice);
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
                        .text(top_adapter_label(&snap))
                        .font_size(10.)
                        .color(palette.muted),
                ),
        )
        .into()
}

fn top_adapter_label(snap: &NetworkSnapshot) -> String {
    if snap.interfaces.is_empty() {
        return "No adapters".into();
    }
    snap.interfaces
        .iter()
        .max_by(|a, b| {
            a.combined_bps
                .partial_cmp(&b.combined_bps)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|i| {
            format!(
                "Top: {} {}",
                crate::adapters::adapter_title(&i.name),
                format_rate(i.combined_bps)
            )
        })
        .unwrap_or_else(|| "No adapters".into())
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
        .with_icon(traffic_icon())
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
            focus_main_window(&mut ctx);
            open_about_window(&mut ctx);
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
            .with_size(340., 200.)
            .with_decorations(false)
            .with_background(Palette::default().panel),
    );
    POPOVER_ID.with(|c| c.set(Some(id)));
}

/// Open (or focus) the dedicated About window from UI actions.
pub fn request_about_window() {
    post_to_renderer(open_about_window);
}

fn open_about_window(ctx: &mut RendererContext) {
    let existing = ctx.windows.iter().find_map(|(id, app)| {
        if app.window().title().contains("About Osman") {
            Some(*id)
        } else {
            None
        }
    });

    if let Some(id) = existing {
        if let Some(app) = ctx.windows.get_mut(&id) {
            app.window().set_visible(true);
            app.window().focus_window();
        }
        return;
    }

    ctx.launch_window(
        WindowConfig::new(crate::about::about_window)
            .with_title("About Osman")
            .with_size(460., 620.)
            .with_min_size(420., 520.)
            .with_background(Palette::default().bg),
    );
}

fn focus_main_window(ctx: &mut RendererContext) {
    let target = ctx.windows.iter().find_map(|(id, app)| {
        let title = app.window().title();
        if title.contains("Osman Mini") {
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

fn menubar_title(rx_bps: f64, tx_bps: f64) -> String {
    let total = rx_bps + tx_bps;
    if total < 1.0 {
        "Osman".into()
    } else {
        format_rate(total)
    }
}

fn menubar_tooltip(rx_bps: f64, tx_bps: f64) -> String {
    let total = rx_bps + tx_bps;
    if total < 1.0 {
        "Osman by NT\nNo traffic".into()
    } else {
        format!(
            "Osman by NT\n↓ Receive {} (blue)\n↑ Send {} (green)\nTotal {}",
            format_rate(rx_bps),
            format_rate(tx_bps),
            format_rate(total),
        )
    }
}

fn traffic_icon() -> Icon {
    const W: u32 = 22;
    const H: u32 = 22;
    let mut rgba = vec![0u8; (W * H * 4) as usize];
    fill_circle(&mut rgba, W, H, 7.0, 11.0, 4.5, [59, 130, 246, 255]);
    fill_circle(&mut rgba, W, H, 15.0, 11.0, 4.5, [34, 197, 94, 255]);
    Icon::from_rgba(rgba, W, H).expect("menubar icon")
}

fn fill_circle(
    rgba: &mut [u8],
    width: u32,
    height: u32,
    cx: f32,
    cy: f32,
    radius: f32,
    color: [u8; 4],
) {
    let r2 = radius * radius;
    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 + 0.5 - cx;
            let dy = y as f32 + 0.5 - cy;
            if dx * dx + dy * dy <= r2 {
                let i = ((y * width + x) * 4) as usize;
                rgba[i..i + 4].copy_from_slice(&color);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::{request_about_window, set_renderer_dispatch_for_test};

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

//! Embedded clinical-scope app icons for window + menubar.

use freya::prelude::LaunchConfig;
use freya::tray::Icon as TrayIcon;
use freya::winit::window::Icon as WindowIcon;

pub const MENUBAR_ICON_BYTES: &[u8] = include_bytes!("../resources/icon/MenubarIcon-22.png");
pub const WINDOW_ICON_BYTES: &[u8] = include_bytes!("../resources/icon/WindowIcon-128.png");
/// 512px clinical-scope PNG for macOS Dock (`setApplicationIconImage`).
pub const DOCK_ICON_BYTES: &[u8] =
    include_bytes!("../resources/icon/AppIcon.appiconset/Icon-512.png");

pub fn menubar_icon() -> TrayIcon {
    LaunchIcon::tray(MENUBAR_ICON_BYTES)
}

pub fn window_icon() -> WindowIcon {
    LaunchIcon::window(WINDOW_ICON_BYTES)
}

/// Freya launch icon loaders — thin wrapper so tests stay decoupled from winit types.
pub struct LaunchIcon;

impl LaunchIcon {
    pub fn tray(bytes: &[u8]) -> TrayIcon {
        LaunchConfig::tray_icon(bytes)
    }

    pub fn window(bytes: &[u8]) -> WindowIcon {
        LaunchConfig::window_icon(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menubar_icon_png_is_valid() {
        assert!(MENUBAR_ICON_BYTES.len() > 8);
        assert_eq!(&MENUBAR_ICON_BYTES[..8], b"\x89PNG\r\n\x1a\n");
    }

    #[test]
    fn dock_icon_png_is_valid() {
        assert!(DOCK_ICON_BYTES.len() > 8);
        assert_eq!(&DOCK_ICON_BYTES[..8], b"\x89PNG\r\n\x1a\n");
    }

    #[test]
    fn window_icon_png_is_valid() {
        assert!(WINDOW_ICON_BYTES.len() > 8);
        assert_eq!(&WINDOW_ICON_BYTES[..8], b"\x89PNG\r\n\x1a\n");
    }

    #[test]
    fn menubar_icon_bytes_match_disk() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources/icon/MenubarIcon-22.png");
        let disk = std::fs::read(&path).expect("MenubarIcon-22.png on disk");
        assert_eq!(disk, MENUBAR_ICON_BYTES);
    }

    #[test]
    fn window_icon_bytes_match_disk() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("resources/icon/WindowIcon-128.png");
        let disk = std::fs::read(&path).expect("WindowIcon-128.png on disk");
        assert_eq!(disk, WINDOW_ICON_BYTES);
    }

    #[test]
    fn app_icns_exists_after_build() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/icon/AppIcon.icns");
        assert!(
            path.is_file(),
            "AppIcon.icns missing — run cargo build to generate from clinical master"
        );
        assert!(std::fs::metadata(path).expect("stat icns").len() > 1024);
    }
}

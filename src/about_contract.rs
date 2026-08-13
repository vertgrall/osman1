//! About branding regression guards.
//!
//! ## Policy — any change touching the About path
//! If your diff touches any of these, extend tests here or in `about_test_harness`
//! and run `./scripts/check-about.sh` before merge:
//!
//! - `src/about*.rs`, `about_assets`, `about_art`
//! - `menubar::request_about_window`, `launch_about_window`
//! - Settings **About Osman…** button, tray menu, macOS app menu
//! - `resources/brand/*` splash / brand PNGs
//!
//! Minimum: source-contract assertion + headless launch check (`launch_checks`).

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    fn manifest_path(rel: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
    }

    fn read_src(rel: &str) -> String {
        std::fs::read_to_string(manifest_path(rel))
            .unwrap_or_else(|err| panic!("could not read {rel}: {err}"))
    }

    fn code_lines(src: &str) -> String {
        src.lines()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.starts_with("//")
                    && !trimmed.starts_with("//!")
                    && !trimmed.starts_with('*')
                    && !trimmed.starts_with("///")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn assert_forbidden(haystack: &str, needle: &str, reason: &str) {
        assert!(
            !haystack.contains(needle),
            "About regression: found forbidden `{needle}` — {reason}"
        );
    }

    fn assert_forbidden_code(haystack: &str, needle: &str, reason: &str) {
        assert_forbidden(&code_lines(haystack), needle, reason);
    }

    fn assert_required(haystack: &str, needle: &str, reason: &str) {
        assert!(
            haystack.contains(needle),
            "About regression: missing required `{needle}` — {reason}"
        );
    }

    #[test]
    fn about_window_launch_regression() {
        use crate::about_test_harness::launch_checks::{
            assert_about_window_launches_ok, snapshot_about_window_launch,
        };

        assert_about_window_launches_ok(&snapshot_about_window_launch());
    }

    #[test]
    fn about_window_size_matches_menubar_config() {
        use crate::about_test_harness::launch_checks::assert_window_constants_match_menubar_config;

        assert_window_constants_match_menubar_config();
    }

    #[test]
    fn settings_routes_about_through_request_about_window() {
        let settings_rs = read_src("src/settings.rs");
        assert_required(
            &settings_rs,
            "menubar::request_about_window()",
            "Settings About button must launch the About window",
        );
        assert_forbidden(
            &settings_rs,
            ".child(about_content(palette))",
            "Settings must not embed the full About panel (clip/blank regressions)",
        );
    }

    #[test]
    fn main_routes_about_through_request_about_window() {
        let main_rs = read_src("src/main.rs");
        assert_required(
            &main_rs,
            "about_assets::preload()",
            "About PNGs must decode before first window",
        );
        assert_forbidden(
            &main_rs,
            ".child(about_content(palette))",
            "Settings must not embed the full About panel (clip/blank regressions)",
        );
        assert_forbidden(
            &main_rs,
            "about_overlay",
            "About must not use main-window overlay (viewport clipping regressions)",
        );
        assert_forbidden(
            &main_rs,
            "bind_show_about",
            "About overlay binding was removed in favor of a dedicated window",
        );
    }

    #[test]
    fn menubar_launches_small_about_window() {
        let menubar_rs = read_src("src/menubar.rs");
        assert_required(
            &menubar_rs,
            "request_about_window",
            "menubar must expose About window launcher",
        );
        assert_required(
            &menubar_rs,
            "launch_about_window",
            "About requests must open/focus the About window on the renderer thread",
        );
        assert_required(
            &menubar_rs,
            "WindowConfig::new(about_window)",
            "About must use the dedicated about_window component",
        );
        assert_required(
            &menubar_rs,
            ".with_title(\"About Osman\")",
            "About window must use the branded title",
        );
        assert_required(
            &menubar_rs,
            "MENU_ABOUT",
            "tray menu must define About item id",
        );
        assert_required(
            &menubar_rs,
            "menu_event.id == MENU_ABOUT",
            "tray About menu item must call request_about_window",
        );
        assert_required(
            &menubar_rs,
            "about_assets::preload()",
            "About window launch must preload branding PNGs",
        );
        assert_required(
            &menubar_rs,
            ".with_resizable(false)",
            "About window must remain fixed-size",
        );
        assert_forbidden(
            &menubar_rs,
            "about_overlay",
            "menubar must not mount About overlay in the main window",
        );
    }

    #[test]
    fn about_panel_uses_canvas_skia_blits() {
        let about_rs = read_src("src/about.rs");
        assert_required(
            &about_rs,
            "draw_about_splash_card",
            "splash must draw through Skia canvas (same path as live charts)",
        );
        assert_required(
            &about_rs,
            "draw_about_brand_mark",
            "brand mark must draw through Skia canvas",
        );
        assert_required(
            &about_rs,
            "pub fn about_window",
            "About must expose dedicated window entry point",
        );
        assert_required(
            &about_rs,
            "ABOUT_WINDOW_W",
            "About window width must be a shared constant",
        );
        assert_required(
            &about_rs,
            "ABOUT_WINDOW_H",
            "About window height must be a shared constant",
        );
        assert_forbidden(
            &about_rs,
            "pub fn about_overlay",
            "About overlay was removed — use about_window instead",
        );
        assert_forbidden(
            &about_rs,
            "Layer::Overlay",
            "About must not render as a main-window overlay",
        );
        assert_forbidden_code(
            &about_rs,
            "image(SPLASH",
            "Freya image() showed blue-folder placeholders in live About UI",
        );
        assert_forbidden_code(
            &about_rs,
            "image(BRAND",
            "Freya image() showed blue-folder placeholders in live About UI",
        );
        assert_forbidden_code(
            &about_rs,
            "ImageViewer::",
            "async ImageViewer broke About branding in production",
        );
    }

    #[test]
    fn macos_app_menu_redirects_to_branded_window() {
        let menu_rs = read_src("src/macos_about_menu.rs");
        assert_required(
            &menu_rs,
            "request_about_window()",
            "App menu About must open branded About window",
        );
        assert_forbidden_code(
            &menu_rs,
            "orderFrontStandardAboutPanel",
            "must not leave the system blue-folder About panel wired",
        );
    }

    #[test]
    fn onboarding_about_uses_window_dispatch() {
        let main_rs = read_src("src/main.rs");
        assert_required(
            &main_rs,
            "menubar::request_about_window()",
            "onboarding About must launch the dedicated window",
        );
        let onboarding_rs = read_src("src/onboarding.rs");
        assert_required(
            &onboarding_rs,
            "on_open_about",
            "onboarding must link to About",
        );
    }

    #[test]
    fn branding_assets_exist_on_disk() {
        for rel in [
            "resources/brand/SplashTowerVillage.png",
            "resources/brand/NewTowerBrandMark.png",
        ] {
            let path = manifest_path(rel);
            assert!(path.is_file(), "missing branding asset: {}", path.display());
            let bytes = std::fs::read(&path).expect("read asset");
            assert_eq!(
                &bytes[..8],
                b"\x89PNG\r\n\x1a\n",
                "{} is not a PNG",
                path.display()
            );
        }
    }

    #[test]
    fn about_pixel_pipeline_still_paints_branding() {
        use crate::about_test_harness::{
            assert_brand_mark_loads, assert_splash_card_loads, render_about_brand_mark,
            render_about_splash_card,
        };

        crate::about_assets::preload();
        assert_splash_card_loads(&render_about_splash_card());
        assert_brand_mark_loads(&render_about_brand_mark());
    }
}

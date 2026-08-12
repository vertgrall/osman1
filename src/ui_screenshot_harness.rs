//! Headless Freya renders of the live app UI — README screenshots of Osman running.

#[cfg(test)]
mod tests {
    use freya::prelude::*;
    use freya_testing::prelude::*;

    use crate::about::about_window;
    use crate::about_assets;

    fn click_nav_item(test: &mut TestingRunner, label: &str) {
        let mut targets = test.find_many(|node, element| {
            Label::try_downcast(element).and_then(|l| {
                (l.text == label).then(|| {
                    let area = node.layout().area;
                    (
                        area.origin.y,
                        (
                            (area.origin.x + area.width() / 2.) as f64,
                            (area.origin.y + area.height() / 2.) as f64,
                        ),
                    )
                })
            })
        });
        targets.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let (_, target) = targets
            .last()
            .unwrap_or_else(|| panic!("nav item not found: {label}"));
        test.click_cursor(*target);
        test.sync_and_update();
    }

    /// `EXPORT_README=1 cargo test ui_screenshot_harness::tests::export_running_ui_screenshots -- --ignored --exact`
    #[test]
    #[ignore = "manual docs export"]
    fn export_running_ui_screenshots() {
        if std::env::var("EXPORT_README").ok().as_deref() != Some("1") {
            return;
        }

        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs/screenshots");
        std::fs::create_dir_all(&dir).expect("create docs/screenshots");
        about_assets::preload();

        let mut overview =
            TestingRunner::new(crate::app_demo, Size2D::new(1400., 920.), |_| {}, 1.0).0;
        overview.sync_and_update();
        overview.render_to_file(dir.join("osman-overview-mock.png"));

        let mut about =
            TestingRunner::new(about_window, Size2D::new(460., 620.), |_| {}, 1.0).0;
        about.sync_and_update();
        about.render_to_file(dir.join("osman-about-window-mock.png"));

        let mut settings =
            TestingRunner::new(crate::app_demo, Size2D::new(1400., 920.), |_| {}, 1.0).0;
        settings.sync_and_update();
        click_nav_item(&mut settings, "Settings");
        settings.render_to_file(dir.join("osman-settings-mock.png"));

        let mut connections =
            TestingRunner::new(crate::app_demo, Size2D::new(1400., 920.), |_| {}, 1.0).0;
        connections.sync_and_update();
        click_nav_item(&mut connections, "Connections");
        connections.render_to_file(dir.join("osman-connections-mock.png"));

        let mut processes =
            TestingRunner::new(crate::app_demo, Size2D::new(1400., 920.), |_| {}, 1.0).0;
        processes.sync_and_update();
        click_nav_item(&mut processes, "Processes");
        processes.render_to_file(dir.join("osman-processes-mock.png"));
    }
}

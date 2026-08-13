//! macOS activation policy — menubar-only mode hides the Dock icon.

#[cfg(target_os = "macos")]
mod platform {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};

    pub fn set_menubar_only(enabled: bool) {
        let Some(mtm) = MainThreadMarker::new() else {
            eprintln!("Osman: menubar-only toggle requires main thread");
            return;
        };
        let app = NSApplication::sharedApplication(mtm);
        let policy = if enabled {
            NSApplicationActivationPolicy::Accessory
        } else {
            NSApplicationActivationPolicy::Regular
        };
        unsafe {
            app.setActivationPolicy(policy);
        }
    }
}

#[cfg(target_os = "macos")]
pub use platform::set_menubar_only;

#[cfg(not(target_os = "macos"))]
pub fn set_menubar_only(_enabled: bool) {}

#[cfg(test)]
mod tests {
    #[test]
    fn activation_module_compiles_on_all_targets() {
        #[cfg(not(target_os = "macos"))]
        super::set_menubar_only(false);
    }
}

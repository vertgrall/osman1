//! Set the macOS Dock icon at runtime (required for `cargo run`; also refreshes bundled apps).

#[cfg(target_os = "macos")]
mod platform {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSImage};
    use objc2_foundation::NSData;

    pub fn install(png_bytes: &'static [u8]) {
        let mtm = MainThreadMarker::new().expect("dock icon install requires main thread");
        let data = NSData::with_bytes(png_bytes);
        let Some(image) = NSImage::initWithData(mtm.alloc(), &data) else {
            eprintln!("Osman: failed to decode dock icon PNG ({} bytes)", png_bytes.len());
            return;
        };

        let app = NSApplication::sharedApplication(mtm);
        unsafe {
            app.setApplicationIconImage(Some(&image));
        }
    }
}

#[cfg(target_os = "macos")]
pub use platform::install;

#[cfg(not(target_os = "macos"))]
pub fn install(_png_bytes: &'static [u8]) {}

#[cfg(test)]
mod tests {
    #[test]
    fn dock_icon_module_compiles_on_all_targets() {
        #[cfg(not(target_os = "macos"))]
        super::install(&[]);
    }
}

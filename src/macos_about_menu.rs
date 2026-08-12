//! Redirect the macOS App menu **About** item to our New Tower branded window.
//!
//! winit wires `About …` to `orderFrontStandardAboutPanel:` (generic blue-folder icon).
//! Mohawk replaces that with `AboutMohawkSheet`; we hook the same menu entry here.

#[cfg(target_os = "macos")]
mod platform {
    use std::cell::RefCell;

    use objc2::rc::{Allocated, Retained};
    use objc2::runtime::AnyObject;
    use objc2::{define_class, msg_send, ClassType, MainThreadMarker, MainThreadOnly};
    use objc2_app_kit::NSApplication;
    use objc2_foundation::{NSObject, NSObjectProtocol};

    define_class!(
        #[unsafe(super(NSObject))]
        #[thread_kind = MainThreadOnly]
        #[name = "OsmanAboutMenuHandler"]
        #[ivars = ()]
        struct AboutMenuHandler;

        impl AboutMenuHandler {
            #[unsafe(method_id(init))]
            fn init(this: Allocated<Self>) -> Retained<Self> {
                let this = this.set_ivars(());
                unsafe { msg_send![super(this), init] }
            }

            #[unsafe(method(showOsmanAbout:))]
            fn show_about(&self, _sender: Option<&AnyObject>) {
                crate::menubar::request_about_window();
            }
        }

        unsafe impl NSObjectProtocol for AboutMenuHandler {}
    );

    thread_local! {
        static HANDLER: RefCell<Option<Retained<AboutMenuHandler>>> = const { RefCell::new(None) };
    }

    fn handler() -> Retained<AboutMenuHandler> {
        let _mtm = MainThreadMarker::new().expect("macOS About redirect requires main thread");
        HANDLER.with(|cell| {
            let mut slot = cell.borrow_mut();
            if slot.is_none() {
                *slot = Some(unsafe { msg_send![AboutMenuHandler::class(), new] });
            }
            slot.as_ref().unwrap().clone()
        })
    }

    pub fn install() {
        let mtm = MainThreadMarker::new().expect("macOS About redirect requires main thread");
        let handler = handler();

        let app = NSApplication::sharedApplication(mtm);
        let Some(menubar) = app.mainMenu() else {
            return;
        };
        let Some(app_menu_item) = menubar.itemAtIndex(0) else {
            return;
        };
        let Some(app_menu) = app_menu_item.submenu() else {
            return;
        };
        let Some(about_item) = app_menu.itemAtIndex(0) else {
            return;
        };

        unsafe {
            about_item.setTarget(Some(&handler));
            about_item.setAction(Some(objc2::sel!(showOsmanAbout:)));
        }
    }
}

#[cfg(target_os = "macos")]
pub use platform::install;

#[cfg(not(target_os = "macos"))]
pub fn install() {}

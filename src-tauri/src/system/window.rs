use cocoa::appkit::NSWindowTitleVisibility;
use tauri::WebviewWindow;

use crate::error::SystemError;

pub struct WindowStyler {}

impl WindowStyler {
    pub fn setup_window_style(window: WebviewWindow) -> Result<(), SystemError> {
        #[cfg(target_os = "macos")]
        {
            use cocoa::appkit::{NSWindow, NSWindowStyleMask};

            let ns_window = window
                .ns_window()
                .map_err(|_| SystemError::Window("Failed to get NS window".to_string()))?;

            unsafe {
                let ns_window = ns_window as cocoa::base::id;
                NSWindow::setTitlebarAppearsTransparent_(ns_window, cocoa::base::YES);

                let mut style_mask = ns_window.styleMask();
                style_mask.set(NSWindowStyleMask::NSFullSizeContentViewWindowMask, true);
                style_mask.remove(
                    NSWindowStyleMask::NSClosableWindowMask
                        | NSWindowStyleMask::NSMiniaturizableWindowMask
                        | NSWindowStyleMask::NSResizableWindowMask,
                );
                ns_window.setStyleMask_(style_mask);
                ns_window.setTitleVisibility_(NSWindowTitleVisibility::NSWindowTitleHidden);

                ns_window.setTitlebarAppearsTransparent_(cocoa::base::YES);
            }
        }
        Ok(())
    }
}

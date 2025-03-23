use crate::core::error::{AppError, SystemError};
use tauri::{App, AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

#[cfg(target_os = "macos")]
use cocoa::appkit::{NSWindow, NSWindowStyleMask, NSWindowTitleVisibility};

pub struct WindowManager {}

impl WindowManager {
    pub fn new() -> Self {
        Self {}
    }

    pub fn setup_windows(&self, app: &App) -> Result<(), AppError> {
        self.create_settings_window(app)?;
        self.create_main_window(app)?;
        Ok(())
    }

    fn create_settings_window(&self, app: &App) -> Result<(), AppError> {
        // Settings window
        let settings_win_builder =
            WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("settings".into()))
                .title("Rune Settings")
                .visible(false)
                .inner_size(800.0, 800.0)
                .hidden_title(true);

        let settings_window = settings_win_builder.build()?;

        // Apply the appropriate style for the settings window
        self.style_settings_window(settings_window)
            .map_err(|e| AppError::Generic(format!("Failed to style settings window: {}", e)))?;

        Ok(())
    }

    fn create_main_window(&self, app: &App) -> Result<(), AppError> {
        // Calculate window position
        let (x_pos, y_pos) = self.calculate_centered_position(app, 150.0, 40.0);

        // Main window
        let win_builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
            .title("Rune")
            .inner_size(150.0, 40.0)
            .position(x_pos, y_pos)
            .visible(false)
            .transparent(true)
            .shadow(false)
            .decorations(false)
            .always_on_top(true);

        let main_window = win_builder.build()?;

        // Apply the appropriate style for the main window
        self.style_main_window(main_window)
            .map_err(|e| AppError::Generic(format!("Failed to style main window: {}", e)))?;

        Ok(())
    }

    fn calculate_centered_position(&self, app: &App, width: f64, height: f64) -> (f64, f64) {
        let monitor = app.primary_monitor().unwrap().unwrap();
        let scale_factor = monitor.scale_factor();
        let monitor_size = monitor.size();

        let logical_width = ((monitor_size.width as f64 / scale_factor) / 2.0) - (width / 2.0);
        let logical_height = (monitor_size.height as f64 / scale_factor) - (height + 80.0);

        (logical_width, logical_height)
    }

    fn style_main_window(&self, window: WebviewWindow) -> Result<(), SystemError> {
        self.remove_titlebar_and_traffic_lights(window)
    }

    fn style_settings_window(&self, window: WebviewWindow) -> Result<(), SystemError> {
        self.remove_titlebar(window)
    }

    // Utility methods for styling windows
    fn remove_titlebar_and_traffic_lights(&self, window: WebviewWindow) -> Result<(), SystemError> {
        #[cfg(target_os = "macos")]
        {
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

    fn remove_titlebar(&self, window: WebviewWindow) -> Result<(), SystemError> {
        #[cfg(target_os = "macos")]
        {
            let ns_window = window
                .ns_window()
                .map_err(|_| SystemError::Window("Failed to get NS window".to_string()))?;

            unsafe {
                let ns_window = ns_window as cocoa::base::id;
                NSWindow::setTitlebarAppearsTransparent_(ns_window, cocoa::base::YES);

                let mut style_mask = ns_window.styleMask();
                style_mask.set(NSWindowStyleMask::NSFullSizeContentViewWindowMask, true);
                // Make sure we keep the resizable mask
                style_mask.set(NSWindowStyleMask::NSResizableWindowMask, true);

                ns_window.setStyleMask_(style_mask);
                ns_window.setTitleVisibility_(NSWindowTitleVisibility::NSWindowTitleHidden);
                ns_window.setTitlebarAppearsTransparent_(cocoa::base::YES);
            }
        }
        Ok(())
    }

    pub fn show_settings_window(&self, app_handle: &AppHandle) -> Result<(), AppError> {
        if let Some(settings_window) = app_handle.get_webview_window("settings") {
            settings_window.show()?;
            settings_window.set_focus()?;
            Ok(())
        } else {
            Err(AppError::Generic("Settings window not found".to_string()))
        }
    }

    pub fn show_main_window(&self, app_handle: &AppHandle) -> Result<(), AppError> {
        if let Some(main_window) = app_handle.get_webview_window("main") {
            main_window.show()?;
            main_window.set_focus()?;
            Ok(())
        } else {
            Err(AppError::Generic("Main window not found".to_string()))
        }
    }
}

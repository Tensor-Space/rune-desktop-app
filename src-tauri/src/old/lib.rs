pub mod audio_devices;
pub mod audio_recorder;
pub mod audio_transcriber;
pub mod permissions;
pub mod shortcuts;
pub mod text_injector;

use std::path::Path;

use cocoa::appkit::{NSWindow, NSWindowStyleMask, NSWindowTitleVisibility};
use cocoa::base::id;
use tauri::{WebviewUrl, WebviewWindowBuilder};

fn setup_window_style(
    window: &tauri::WebviewWindow,
    title_transparent: bool,
    remove_tool_bar: bool,
) {
    #[cfg(target_os = "macos")]
    {
        let ns_window = window.ns_window().unwrap() as id;
        unsafe {
            NSWindow::setTitlebarAppearsTransparent_(ns_window, cocoa::base::YES);
            let mut style_mask = ns_window.styleMask();
            style_mask.set(
                NSWindowStyleMask::NSFullSizeContentViewWindowMask,
                title_transparent,
            );

            if remove_tool_bar {
                style_mask.remove(
                    NSWindowStyleMask::NSClosableWindowMask
                        | NSWindowStyleMask::NSMiniaturizableWindowMask
                        | NSWindowStyleMask::NSResizableWindowMask,
                );
            }

            ns_window.setStyleMask_(style_mask);

            ns_window.setTitleVisibility_(if title_transparent {
                NSWindowTitleVisibility::NSWindowTitleHidden
            } else {
                NSWindowTitleVisibility::NSWindowTitleVisible
            });

            ns_window.setTitlebarAppearsTransparent_(if title_transparent {
                cocoa::base::YES
            } else {
                cocoa::base::NO
            });
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            audio_devices::get_audio_devices,
            audio_devices::set_default_device,
            audio_devices::get_default_device,
            permissions::check_accessibility_permissions,
            permissions::request_accessibility_permissions,
        ]);

    builder
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            {
                if !macos_accessibility_client::accessibility::application_is_trusted() {
                    // Request permissions if not granted
                    macos_accessibility_client::accessibility::application_is_trusted_with_prompt();
                }
            }

            let settings_win_builder = WebviewWindowBuilder::new(
                app,
                "settings",
                WebviewUrl::App(Path::new("settings").to_path_buf()),
            )
            .title("Rune Settings")
            .inner_size(800.0, 800.0);

            settings_win_builder.build().unwrap();

            // setup_window_style(&settings_window, true, false);

            let win_builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
                .title("Rune")
                .inner_size(400.0, 80.0)
                .position(
                    {
                        let monitor = app.primary_monitor().unwrap().unwrap();
                        let scale_factor = monitor.scale_factor();
                        let monitor_size = monitor.size();
                        println!(
                            "Monitor size: {:?}, Scale factor: {}",
                            monitor_size, scale_factor
                        );

                        // Convert to logical pixels
                        let logical_width =
                            (monitor_size.width as f64 / scale_factor) - (400.0 + 20.0);
                        logical_width
                    },
                    40.0,
                )
                .visible(false)
                .shadow(false)
                .title_bar_style(tauri::TitleBarStyle::Transparent)
                .decorations(true);

            let window = win_builder.build().unwrap();

            setup_window_style(&window, true, true);

            #[cfg(desktop)]
            shortcuts::setup_shortcuts(app, window.clone())?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

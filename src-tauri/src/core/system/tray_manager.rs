use crate::{
    core::{app::AppState, error::AppError},
    events::types::RecordingCommand,
};
use std::sync::Arc;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager, Wry,
};

pub struct TrayManager {
    app_state: Arc<AppState>,
}

impl TrayManager {
    pub fn new(app_state: Arc<AppState>) -> Self {
        Self { app_state }
    }

    pub fn setup_tray(&self, app_handle: &AppHandle) -> Result<(), AppError> {
        let tray_menu = Self::build_tray_menu(app_handle, true, false)?;
        let state_clone = self.app_state.clone();

        let _tray = TrayIconBuilder::with_id("tray")
            .icon(
                Image::from_path(
                    app_handle
                        .path()
                        .resource_dir()
                        .unwrap()
                        .join("icons/tray-icon.ico"),
                )
                .unwrap(),
            )
            .menu(&tray_menu)
            .on_menu_event(move |app, event| {
                let app_handle = app.clone();
                let state = state_clone.clone();

                match event.id.as_ref() {
                    "start_recording" => {
                        tauri::async_runtime::spawn(async move {
                            let guard = state.recording_tx.lock().await;
                            if let Some(tx) = guard.as_ref() {
                                let _ = tx.send(RecordingCommand::Start).await;
                            }

                            // Update the menu
                            if let Some(tray) = app_handle.tray_by_id("tray") {
                                if let Ok(new_menu) =
                                    Self::build_tray_menu(&app_handle, false, true)
                                {
                                    let _ = tray.set_menu(Some(new_menu.clone()));
                                    app_handle.set_menu(new_menu).unwrap();
                                }
                            }
                        });
                    }
                    "stop_recording" => {
                        tauri::async_runtime::spawn(async move {
                            let guard = state.recording_tx.lock().await;
                            if let Some(tx) = guard.as_ref() {
                                let _ = tx.send(RecordingCommand::Stop).await;
                            }

                            // Update the menu
                            if let Some(tray) = app_handle.tray_by_id("tray") {
                                if let Ok(new_menu) =
                                    Self::build_tray_menu(&app_handle, true, false)
                                {
                                    let _ = tray.set_menu(Some(new_menu.clone())).unwrap();
                                    app_handle.set_menu(new_menu).unwrap();
                                }
                            }
                        });
                    }
                    "settings" => {
                        if let Some(settings_window) = app_handle.get_webview_window("settings") {
                            let _ = settings_window.show();
                            let _ = settings_window.set_focus();
                        }
                    }
                    "quit" => {
                        app_handle.exit(0);
                    }
                    _ => {}
                }
            })
            .build(app_handle)
            .map_err(|e| AppError::Config(format!("Failed to create tray icon: {}", e).into()))?;

        if let Some(tray_handle) = app_handle.tray_by_id("stop_recording") {
            let _ = tray_handle.set_visible(false);
        } else {
            print!("Stop Recording not found")
        }

        Ok(())
    }

    pub fn build_tray_menu(
        app: &AppHandle,
        start_enabled: bool,
        stop_enabled: bool,
    ) -> Result<Menu<Wry>, AppError> {
        let start_recording_item = MenuItem::with_id(
            app,
            "start_recording",
            "Start Recording",
            start_enabled,
            None::<&str>,
        )
        .map_err(|e| AppError::Config(format!("Failed to create menu item: {}", e).into()))?;

        let stop_recording_item = MenuItem::with_id(
            app,
            "stop_recording",
            "Stop Recording",
            stop_enabled,
            None::<&str>,
        )
        .map_err(|e| AppError::Config(format!("Failed to create menu item: {}", e).into()))?;

        let separator = PredefinedMenuItem::separator(app)
            .map_err(|e| AppError::Config(format!("Failed to create separator: {}", e).into()))?;

        let settings_item = MenuItem::with_id(app, "settings", "Rune Settings", true, None::<&str>)
            .map_err(|e| AppError::Config(format!("Failed to create menu item: {}", e).into()))?;

        let quit_item = MenuItem::with_id(app, "quit", "Quit App", true, None::<&str>)
            .map_err(|e| AppError::Config(format!("Failed to create menu item: {}", e).into()))?;

        Menu::with_items(
            app,
            &[
                &start_recording_item,
                &stop_recording_item,
                &separator,
                &settings_item,
                &quit_item,
            ],
        )
        .map_err(|e| AppError::Config(format!("Failed to create menu: {}", e).into()))
    }
}

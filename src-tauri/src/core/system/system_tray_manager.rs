use crate::core::{
    app::AppState,
    error::{AppError, SystemError},
    state_machine::AppCommand,
    utils::updater::check_for_updates,
};
use std::sync::Arc;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager,
};
use tauri_plugin_notification::NotificationExt;

pub struct SystemTrayManager {
    app_handle: AppHandle,
    app_state: Arc<AppState>,
}

impl SystemTrayManager {
    pub fn new(state: Arc<AppState>, app_handle: AppHandle) -> Result<Self, AppError> {
        Ok(Self {
            app_handle,
            app_state: state,
        })
    }

    pub fn setup(&self) -> Result<(), AppError> {
        let tray_menu = self.build_tray_menu(true, false)?;
        let app_state = Arc::clone(&self.app_state);

        let _tray = TrayIconBuilder::with_id("tray")
            .icon(self.load_tray_icon()?)
            .menu(&tray_menu)
            .on_menu_event(move |app, event| {
                Self::handle_tray_menu_event(app, &event.id.as_ref(), &app_state);
            })
            .build(&self.app_handle)
            .map_err(|e| AppError::Config(format!("Failed to create tray icon: {}", e).into()))?;

        if let Some(tray_handle) = self.app_handle.tray_by_id("tray") {
            let _ = tray_handle.set_visible(true);
        } else {
            log::warn!("Tray not found");
        }

        Ok(())
    }

    pub fn handle_tray_menu_event(app: &AppHandle, event_id: &str, app_state: &Arc<AppState>) {
        match event_id {
            "start_recording" => {
                log::info!("Start recording requested from tray");
                if let Some(machine) = &*app_state.state_machine.lock() {
                    machine.send_command(AppCommand::StartRecording);
                }
                if let Some(tray) = app.tray_by_id("tray") {
                    if let Ok(new_menu) = Self::build_tray_menu_static(app, false, true) {
                        let _ = tray.set_menu(Some(new_menu));
                    }
                }
            }
            "stop_recording" => {
                log::info!("Stop recording requested from tray");
                if let Some(machine) = &*app_state.state_machine.lock() {
                    machine.send_command(AppCommand::StopRecording);
                }

                if let Some(tray) = app.tray_by_id("tray") {
                    if let Ok(new_menu) = Self::build_tray_menu_static(app, true, false) {
                        let _ = tray.set_menu(Some(new_menu));
                    }
                }
            }
            "cancel_recording" => {
                log::info!("Cancel recording requested from tray");
                app_state.cancel_current_operation();

                if let Some(tray) = app.tray_by_id("tray") {
                    if let Ok(new_menu) = Self::build_tray_menu_static(app, true, false) {
                        let _ = tray.set_menu(Some(new_menu));
                    }
                }
            }
            "settings" => {
                if let Some(settings_window) = app.get_webview_window("settings") {
                    if let Err(e) = settings_window.show() {
                        log::error!("Failed to show settings window: {}", e);
                    }
                    if let Err(e) = settings_window.set_focus() {
                        log::error!("Failed to focus settings window: {}", e);
                    }
                }
            }
            "history" => {
                if let Some(history_window) = app.get_webview_window("history") {
                    if let Err(e) = history_window.show() {
                        log::error!("Failed to show history window: {}", e);
                    }
                    if let Err(e) = history_window.set_focus() {
                        log::error!("Failed to focus history window: {}", e);
                    }
                }
            }
            "check_updates" => {
                log::info!("Check for updates requested from tray");

                let app_handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    match check_for_updates(app_handle.clone()).await {
                        Ok(_update_found) => {}
                        Err(e) => {
                            log::error!("Failed to check for updates: {}", e);

                            app_handle
                                .notification()
                                .builder()
                                .title("Rune")
                                .body(&format!("Update check failed: {}", e))
                                .show()
                                .unwrap_or_else(|e| {
                                    log::error!("Failed to show notification: {}", e)
                                });
                        }
                    }
                });
            }
            "quit" => {
                log::info!("Quit requested from tray");
                app_state.cancel_current_operation();

                std::thread::sleep(std::time::Duration::from_millis(200));

                app.exit(0);
            }
            _ => {}
        }
    }

    pub fn update_menu(&self, start_enabled: bool, stop_enabled: bool) -> Result<(), AppError> {
        if let Some(tray) = self.app_handle.tray_by_id("tray") {
            let new_menu = self.build_tray_menu(start_enabled, stop_enabled)?;
            tray.set_menu(Some(new_menu))
                .map_err(|e| AppError::System(SystemError::General(e.to_string())))?;
        }
        Ok(())
    }

    fn load_tray_icon(&self) -> Result<Image, AppError> {
        let icon_path = self
            .app_handle
            .path()
            .resource_dir()
            .map_err(|_| AppError::Generic("Failed to get resource directory".into()))?
            .join("icons/tray-icon.ico");

        if !icon_path.exists() {
            log::warn!("Tray icon not found at: {:?}", icon_path);
        }

        Image::from_path(icon_path)
            .map_err(|e| AppError::Config(format!("Failed to load tray icon: {}", e).into()))
    }

    fn build_tray_menu(
        &self,
        start_enabled: bool,
        stop_enabled: bool,
    ) -> Result<Menu<tauri::Wry>, AppError> {
        Self::build_tray_menu_static(&self.app_handle, start_enabled, stop_enabled)
    }

    pub fn build_tray_menu_static(
        app: &AppHandle,
        start_enabled: bool,
        stop_enabled: bool,
    ) -> Result<Menu<tauri::Wry>, AppError> {
        let start_recording_item =
            Self::create_menu_item(app, "start_recording", "Start Recording", start_enabled)?;
        let stop_recording_item =
            Self::create_menu_item(app, "stop_recording", "Stop Recording", stop_enabled)?;
        let cancel_recording_item =
            Self::create_menu_item(app, "cancel_recording", "Cancel Recording", stop_enabled)?;
        let history_item = Self::create_menu_item(app, "history", "History", true)?;
        let separator = PredefinedMenuItem::separator(app)
            .map_err(|e| AppError::Config(format!("Failed to create separator: {}", e).into()))?;

        let version = app.package_info().version.to_string();
        let version_item =
            Self::create_menu_item(app, "version", &format!("Version: {}", version), false)?;
        let check_updates_item =
            Self::create_menu_item(app, "check_updates", "Check for Updates", true)?;
        let settings_item = Self::create_menu_item(app, "settings", "Settings", true)?;
        let quit_item = Self::create_menu_item(app, "quit", "Quit App", true)?;

        Menu::with_items(
            app,
            &[
                &start_recording_item,
                &stop_recording_item,
                &cancel_recording_item,
                &separator,
                &history_item,
                &settings_item,
                &separator,
                &check_updates_item,
                &quit_item,
                &separator,
                &version_item,
            ],
        )
        .map_err(|e| AppError::Config(format!("Failed to create menu: {}", e).into()))
    }

    fn create_menu_item(
        app: &AppHandle,
        id: &str,
        label: &str,
        enabled: bool,
    ) -> Result<MenuItem<tauri::Wry>, AppError> {
        MenuItem::with_id(app, id, label, enabled, None::<&str>).map_err(|e| {
            AppError::Config(format!("Failed to create menu item '{}': {}", id, e).into())
        })
    }
}

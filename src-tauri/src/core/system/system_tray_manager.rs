use crate::{
    controllers::audio_pipleine_controller::AudioPipelineController,
    core::{
        app::AppState,
        error::{AppError, SystemError},
    },
};
use std::sync::Arc;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Listener, Manager,
};

pub struct SystemTrayManager {
    app_handle: AppHandle,
    recording_pipeline: Arc<AudioPipelineController>,
    app_state: Arc<AppState>,
}

impl SystemTrayManager {
    pub fn new(state: Arc<AppState>, app_handle: AppHandle) -> Result<Self, AppError> {
        let recording_pipeline = match &*state.audio_pipeline.lock() {
            Some(pipeline) => Arc::clone(pipeline),
            None => {
                return Err(AppError::Generic(
                    "Audio pipeline not initialized".to_string(),
                ))
            }
        };

        Ok(Self {
            app_handle,
            recording_pipeline,
            app_state: state,
        })
    }

    pub fn setup(&self) -> Result<(), AppError> {
        let tray_menu = self.build_tray_menu(true, false)?;
        let recording_pipeline = Arc::clone(&self.recording_pipeline);
        let app_state = Arc::clone(&self.app_state);

        let _tray = TrayIconBuilder::with_id("tray")
            .icon(self.load_tray_icon()?)
            .menu(&tray_menu)
            .on_menu_event(move |app, event| {
                Self::handle_tray_menu_event(
                    app,
                    &event.id.as_ref(),
                    &recording_pipeline,
                    &app_state,
                );
            })
            .build(&self.app_handle)
            .map_err(|e| AppError::Config(format!("Failed to create tray icon: {}", e).into()))?;

        if let Some(tray_handle) = self.app_handle.tray_by_id("tray") {
            let _ = tray_handle.set_visible(true);
        } else {
            log::warn!("Tray not found");
        }

        let recording_pipeline_clone = Arc::clone(&self.recording_pipeline);
        let app_handle_clone = self.app_handle.clone();
        let app_state_clone = Arc::clone(&self.app_state);

        let _handler = self
            .app_handle
            .listen("tauri://close-requested", move |_event| {
                let pipeline = &recording_pipeline_clone;
                app_state_clone.runtime.block_on(pipeline.cancel());

                std::thread::sleep(std::time::Duration::from_millis(100));

                if let Some(window) = app_handle_clone.get_webview_window("main") {
                    let _ = window.close();
                }
            });

        let recording_pipeline_for_cancel = Arc::clone(&self.recording_pipeline);
        let app_state_for_cancel = Arc::clone(&self.app_state);
        let _cancel_handler = self.app_handle.listen("cancel-recording", move |_| {
            app_state_for_cancel
                .runtime
                .block_on(recording_pipeline_for_cancel.cancel());
        });

        Ok(())
    }

    pub fn handle_tray_menu_event(
        app: &AppHandle,
        event_id: &str,
        recording_pipeline: &AudioPipelineController,
        app_state: &Arc<AppState>,
    ) {
        match event_id {
            "start_recording" => {
                if let Ok(_) = app_state.runtime.block_on(recording_pipeline.start()) {
                    if let Some(tray) = app.tray_by_id("tray") {
                        if let Ok(new_menu) = Self::build_tray_menu_static(app, false, true) {
                            let _ = tray.set_menu(Some(new_menu));
                        }
                    }
                }
            }
            "stop_recording" => {
                app_state.runtime.block_on(recording_pipeline.stop());
                if let Some(tray) = app.tray_by_id("tray") {
                    if let Ok(new_menu) = Self::build_tray_menu_static(app, true, false) {
                        let _ = tray.set_menu(Some(new_menu));
                    }
                }
            }
            "cancel_recording" => {
                app_state.runtime.block_on(recording_pipeline.cancel());
                if let Some(tray) = app.tray_by_id("tray") {
                    if let Ok(new_menu) = Self::build_tray_menu_static(app, true, false) {
                        let _ = tray.set_menu(Some(new_menu));
                    }
                }
            }
            "settings" => {
                if let Some(settings_window) = app.get_webview_window("settings") {
                    let _ = settings_window.show();
                    let _ = settings_window.set_focus();
                }
            }
            "history" => {
                if let Some(history_window) = app.get_webview_window("history") {
                    let _ = history_window.show();
                    let _ = history_window.set_focus();
                }
            }
            "quit" => {
                app_state.runtime.block_on(recording_pipeline.cancel());
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
        let settings_item = Self::create_menu_item(app, "settings", "Rune Settings", true)?;
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
                &quit_item,
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

    pub fn get_pipeline(&self) -> Arc<AudioPipelineController> {
        Arc::clone(&self.recording_pipeline)
    }
}

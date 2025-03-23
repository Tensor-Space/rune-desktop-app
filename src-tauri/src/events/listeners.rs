use crate::events::types::{AppEvent, RecordingCommand};
use std::sync::Arc;
use tauri::{AppHandle, Listener};
use tokio::sync::{mpsc::Sender, Mutex};

pub fn setup_app_event_listeners(
    app_handle: &AppHandle,
    cmd_tx: Arc<Mutex<Option<Sender<RecordingCommand>>>>,
) {
    // Clone what we need for the closure
    let app = app_handle.clone();
    let cmd_tx_clone = cmd_tx.clone();

    // Listen for UI-initiated events
    app_handle.listen("app-event", move |event| {
        if let Ok(app_event) = serde_json::from_str::<AppEvent>(event.payload()) {
            let app_clone = app.clone();
            let tx_clone = cmd_tx_clone.clone();
            tauri::async_runtime::spawn(async move {
                handle_app_event(&app_clone, app_event, &tx_clone).await;
            });
        }
    });
}

async fn handle_app_event(
    _app_handle: &AppHandle,
    event: AppEvent,
    cmd_tx: &Arc<Mutex<Option<Sender<RecordingCommand>>>>,
) {
    match event {
        AppEvent::CancelProcessing => {
            let guard = cmd_tx.lock().await;
            if let Some(tx) = &*guard {
                let _ = tx.send(RecordingCommand::Stop).await;
            }
        }
        AppEvent::RetryTranscription => {
            // If we support retrying transcription, implement here
            log::info!("Retry transcription requested, not implemented yet");
        }
        AppEvent::RequestAudioStatus => {
            // Respond with current audio status
            log::info!("Audio status requested, not implemented yet");
        }
    }
}

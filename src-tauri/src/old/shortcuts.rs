use crate::audio_recorder::AudioRecorder;
use crate::audio_transcriber::AudioTranscriber;
use crate::text_injector::TextInjector;
use std::{env::temp_dir, path::Path, process::Command, sync::Arc};
use tauri::App;

use tauri_plugin_global_shortcut::{
    Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutEvent, ShortcutState,
};

pub struct RecordingState {
    recorder: Arc<AudioRecorder>,
    transcriber: Arc<parking_lot::Mutex<AudioTranscriber>>,
    previous_app_name: parking_lot::Mutex<Option<String>>,
    temp_file_path: parking_lot::Mutex<Option<String>>,
}

impl Default for RecordingState {
    fn default() -> Self {
        Self {
            recorder: Arc::new(AudioRecorder::default()),
            transcriber: Arc::new(parking_lot::Mutex::new(
                AudioTranscriber::new().expect("Failed to initialize transcriber"),
            )),
            previous_app_name: parking_lot::Mutex::new(None),
            temp_file_path: parking_lot::Mutex::new(None),
        }
    }
}

impl RecordingState {
    fn new() -> Self {
        Self::default()
    }

    fn get_temp_recording_path() -> std::path::PathBuf {
        let mut temp_path = temp_dir();
        temp_path.push("rune_recording.wav");
        temp_path
    }
}

fn get_frontmost_app_name() -> Option<String> {
    let output = Command::new("osascript")
        .arg("-e")
        .arg(r#"tell application "System Events" to get name of first application process whose frontmost is true"#)
        .output()
        .ok()?;

    String::from_utf8(output.stdout)
        .ok()
        .map(|s| s.trim().to_string())
}

fn activate_app(app_name: &str) {
    Command::new("osascript")
        .arg("-e")
        .arg(format!(r#"tell application "{}" to activate"#, app_name))
        .output()
        .ok();
}

pub fn setup_shortcuts(
    app: &App,
    window: tauri::WebviewWindow,
) -> Result<(), Box<dyn std::error::Error>> {
    let meta_shortcut = Shortcut::new(Some(Modifiers::META), Code::KeyK);
    let esc_shortcut = Shortcut::new(None, Code::Escape);

    let recording_state = Arc::new(RecordingState::new());
    let recording_state_clone = Arc::clone(&recording_state);

    let window_handle = window.clone();

    let handler = move |app: &tauri::AppHandle, shortcut: &Shortcut, event: ShortcutEvent| {
        if shortcut == &meta_shortcut {
            match event.state {
                ShortcutState::Pressed => {
                    println!("Started recording...");
                    if let Some(app_name) = get_frontmost_app_name() {
                        println!("Previous app: {}", app_name);
                        *recording_state_clone.previous_app_name.lock() = Some(app_name);
                    }

                    window_handle.show().unwrap();
                    window_handle.set_focus().unwrap();

                    let temp_path = RecordingState::get_temp_recording_path();
                    *recording_state_clone.temp_file_path.lock() =
                        Some(temp_path.to_str().unwrap().to_string().clone());

                    if let Err(e) = recording_state_clone
                        .recorder
                        .start_recording(app, temp_path)
                    {
                        println!("Failed to start recording: {}", e);
                    }
                }
                ShortcutState::Released => {
                    println!("Stopped recording, transcribing...");
                    window_handle.hide().unwrap();

                    let previous_app = recording_state_clone.previous_app_name.lock().clone();
                    *recording_state_clone.previous_app_name.lock() = None;

                    if let Err(e) = recording_state_clone.recorder.stop_recording() {
                        println!("Failed to stop recording: {}", e);
                        return;
                    }

                    if let Some(app_name) = previous_app {
                        println!("Activating previous app: {}", app_name);
                        activate_app(&app_name);
                    }

                    // Use the stored temporary file path for transcription
                    if let Some(temp_path) = recording_state_clone.temp_file_path.lock().clone() {
                        match recording_state_clone
                            .transcriber
                            .lock()
                            .transcribe(Path::new(&temp_path.clone()).to_path_buf())
                        {
                            Ok(transcription) => {
                                println!("Transcription: {:?}", transcription);

                                if let Some(text) = transcription.first() {
                                    println!("Injecting text: {}", text);
                                    let mut injector = TextInjector::new();
                                    if let Err(e) = injector.inject_text(text) {
                                        println!("Failed to inject text: {}", e);
                                    }
                                }
                            }
                            Err(e) => println!("Transcription error: {}", e),
                        }

                        // Clean up the temporary file
                        if let Err(e) = std::fs::remove_file(&temp_path) {
                            println!("Failed to remove temporary file: {}", e);
                        }
                    }
                }
            }
        } else if shortcut == &esc_shortcut {
            window_handle.hide().unwrap();
            if let Some(ref app_name) = *recording_state_clone.previous_app_name.lock() {
                println!("Activating previous app (ESC): {}", app_name);
                activate_app(app_name);
            }
            *recording_state_clone.previous_app_name.lock() = None;
        }
    };

    app.handle().plugin(
        tauri_plugin_global_shortcut::Builder::new()
            .with_handler(handler)
            .build(),
    )?;

    app.global_shortcut().register(meta_shortcut)?;
    app.global_shortcut().register(esc_shortcut)?;

    Ok(())
}

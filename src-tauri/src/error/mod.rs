mod conversions;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Command error: {0}")]
    Command(#[from] CommandError),

    #[error("Audio error: {0}")]
    Audio(#[from] AudioError),

    #[error("System error: {0}")]
    System(#[from] SystemError),

    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("Tauri error: {0}")]
    Tauri(#[from] tauri::Error),

    #[error("Generic error: {0}")]
    Generic(String),
}

#[derive(Error, Debug)]
pub enum CommandError {
    #[error("Invalid command: {0}")]
    Invalid(String),

    #[error("Command execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Audio error: {0}")]
    Audio(#[from] AudioError),

    #[error("System error: {0}")]
    System(#[from] SystemError),
}

#[derive(Error, Debug)]
pub enum AudioError {
    #[error("Device error: {0}")]
    Device(String),

    #[error("Recording error: {0}")]
    Recording(String),

    #[error("Transcription error: {0}")]
    Transcription(String),
}

#[derive(Error, Debug)]
pub enum SystemError {
    #[error("General error: {0}")]
    General(String),

    #[error("Permission error: {0}")]
    Permission(String),

    #[error("Window error: {0}")]
    Window(String),
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Loading error: {0}")]
    Loading(String),

    #[error("Invalid configuration: {0}")]
    Invalid(String),
}

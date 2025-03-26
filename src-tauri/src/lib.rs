use core::{app::App, error::AppError};
use env_logger::Env;
use log::{info, LevelFilter};

pub mod commands;
pub mod controllers;
pub mod core;
pub mod prompts;
pub mod services;

pub type Result<T> = std::result::Result<T, AppError>;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<()> {
    // Initialize logger with default level INFO
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .format_module_path(true)
        .filter_module("rune", LevelFilter::Debug)
        .init();

    info!("Starting Rune application");

    let app = App::new()?;
    app.run()
}

use core::{app::App, error::AppError};

pub mod commands;
pub mod controllers;
pub mod core;
pub mod prompts;
pub mod services;

pub type Result<T> = std::result::Result<T, AppError>;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<()> {
    let app = App::new()?;
    app.run()
}

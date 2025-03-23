use core::{app::App, error::AppError};

pub mod audio;
pub mod commands;
pub mod core;
pub mod events;
pub mod prompts;
pub mod services;
pub mod text;

pub type Result<T> = std::result::Result<T, AppError>;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<()> {
    env_logger::init();
    let app = App::new()?;
    app.run()
}

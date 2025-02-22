use core::{app::App, error::AppError};

pub mod audio;
pub mod commands;
pub mod handlers;
pub mod io;

pub mod core;

pub type Result<T> = std::result::Result<T, AppError>;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<()> {
    let app = App::new()?;
    app.run()
}

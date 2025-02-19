mod app;
mod audio;
mod commands;
mod config;
mod error;
mod input;
mod system;
mod utils;

pub use app::App;
pub use error::AppError;

pub type Result<T> = std::result::Result<T, crate::error::AppError>;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<()> {
    let app = App::new()?;
    app.run()
}

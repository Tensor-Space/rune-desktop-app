use enigo::{Enigo, Keyboard, Settings};

use crate::core::error::{AppError, SystemError};

pub struct TextInjector {}

impl TextInjector {
    pub fn new() -> Result<Self, AppError> {
        Ok(Self {})
    }

    pub fn inject_text(&self, text: &str) -> Result<(), AppError> {
        Enigo::new(&Settings::default())
            .map_err(|e| AppError::System(SystemError::General(e.to_string())))?
            .text(text)
            .map_err(|e| AppError::System(SystemError::General(e.to_string())))
    }
}

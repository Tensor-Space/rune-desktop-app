use enigo::{Enigo, Keyboard, Settings};

use crate::core::error::{AppError, SystemError};

pub struct TextInjector {
    enigo: Enigo,
}

impl TextInjector {
    pub fn new() -> Result<Self, AppError> {
        Ok(Self {
            enigo: Enigo::new(&Settings::default())
                .map_err(|e| AppError::System(SystemError::General(e.to_string())))?,
        })
    }

    pub fn inject_text(&mut self, text: &str) -> Result<(), AppError> {
        self.enigo
            .text(text)
            .map_err(|e| AppError::System(SystemError::General(e.to_string())))
    }
}

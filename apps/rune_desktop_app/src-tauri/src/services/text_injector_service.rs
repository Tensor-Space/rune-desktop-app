use crate::core::error::{AppError, SystemError};
use enigo::{Enigo, Keyboard, Settings};

pub struct TextInjectorService;

impl TextInjectorService {
    pub fn inject_text(text: &str) -> Result<(), AppError> {
        Enigo::new(&Settings::default())
            .map_err(|e| AppError::System(SystemError::General(e.to_string())))?
            .text(text)
            .map_err(|e| AppError::System(SystemError::General(e.to_string())))
    }
}

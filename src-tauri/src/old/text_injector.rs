use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::error::Error;

pub struct TextInjector {
    enigo: Enigo,
}

impl TextInjector {
    pub fn new() -> Self {
        Self {
            enigo: Enigo::new(&Settings::default()).unwrap(),
        }
    }

    pub fn inject_text(&mut self, text: &str) -> Result<(), Box<dyn Error>> {
        // Send text
        self.enigo.text(text).unwrap();

        Ok(())
    }

    pub fn inject_text_via_clipboard(&mut self, text: &str) -> Result<(), Box<dyn Error>> {
        use clipboard::{ClipboardContext, ClipboardProvider};

        // Store original clipboard content
        let mut ctx: ClipboardContext = ClipboardProvider::new()?;
        let original_contents = ctx.get_contents()?;

        // Set new content
        ctx.set_contents(text.to_owned())?;

        // Simulate paste
        self.enigo.key(Key::Control, Direction::Press).unwrap();
        self.enigo.key(Key::Unicode('v'), Direction::Click).unwrap();
        self.enigo.key(Key::Control, Direction::Release).unwrap();

        // Restore original clipboard content
        ctx.set_contents(original_contents)?;

        Ok(())
    }
}

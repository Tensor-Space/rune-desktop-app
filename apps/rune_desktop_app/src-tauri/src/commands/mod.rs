pub mod audio_commands;
pub mod system_commands;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandError {
    pub message: String,
    pub kind: CommandErrorKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandErrorKind {
    Audio,
    System,
    Permission,
    Invalid,
}

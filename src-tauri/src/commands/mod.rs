pub mod audio;
pub mod system;

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

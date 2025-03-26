use super::*;

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::Generic(err.to_string())
    }
}

impl From<String> for ConfigError {
    fn from(error: String) -> Self {
        ConfigError::Loading(error)
    }
}

impl From<String> for SystemError {
    fn from(s: String) -> Self {
        SystemError::General(s)
    }
}

impl From<&str> for SystemError {
    fn from(s: &str) -> Self {
        SystemError::General(s.to_string())
    }
}

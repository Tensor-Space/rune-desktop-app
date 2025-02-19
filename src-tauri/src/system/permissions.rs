use crate::error::SystemError;

pub fn check_accessibility_permissions() -> Result<bool, SystemError> {
    #[cfg(target_os = "macos")]
    {
        Ok(macos_accessibility_client::accessibility::application_is_trusted())
    }

    #[cfg(not(target_os = "macos"))]
    {
        Ok(true)
    }
}

pub fn request_accessibility_permissions() -> Result<bool, SystemError> {
    #[cfg(target_os = "macos")]
    {
        Ok(macos_accessibility_client::accessibility::application_is_trusted_with_prompt())
    }

    #[cfg(not(target_os = "macos"))]
    {
        Ok(true)
    }
}

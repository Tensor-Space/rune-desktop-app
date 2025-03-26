use crate::core::error::SystemError;
use cpal::{
    self,
    traits::{DeviceTrait, HostTrait},
};

pub struct PermissionManager;

impl PermissionManager {
    pub fn check_accessibility_permissions() -> Result<bool, SystemError> {
        #[cfg(target_os = "macos")]
        return Ok(macos_accessibility_client::accessibility::application_is_trusted());

        #[cfg(not(target_os = "macos"))]
        return Ok(true);
    }

    pub fn request_accessibility_permissions() -> Result<bool, SystemError> {
        #[cfg(target_os = "macos")]
        return Ok(macos_accessibility_client::accessibility::application_is_trusted_with_prompt());

        #[cfg(not(target_os = "macos"))]
        return Ok(true);
    }

    pub fn check_microphone_permissions() -> Result<bool, SystemError> {
        Self::try_access_microphone(false)
    }

    pub fn request_microphone_permissions() -> Result<bool, SystemError> {
        Self::try_access_microphone(true)
    }

    fn try_access_microphone(create_stream: bool) -> Result<bool, SystemError> {
        let host = cpal::default_host();

        let device = match host.default_input_device() {
            Some(device) => device,
            None => return Ok(false),
        };

        if let Err(_) = device.name() {
            return Ok(false);
        }

        if !create_stream {
            return Ok(true);
        }

        let config = match device.default_input_config() {
            Ok(config) => config,
            Err(_) => return Ok(false),
        };

        match device.build_input_stream(&config.into(), |_: &[f32], _| {}, |_| {}, None) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

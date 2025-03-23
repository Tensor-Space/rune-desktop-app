use crate::core::error::SystemError;
use cpal::{
    self,
    traits::{DeviceTrait, HostTrait},
};

#[cfg(target_os = "macos")]
use macos_accessibility_client;

pub struct PermissionManager {}

impl PermissionManager {
    pub fn new() -> Self {
        Self {}
    }

    pub fn check_all_permissions(&self) -> Result<bool, SystemError> {
        let accessibility = self.check_accessibility_permissions()?;
        let microphone = self.check_microphone_permissions()?;

        Ok(accessibility && microphone)
    }

    pub fn request_all_permissions(&self) -> Result<bool, SystemError> {
        let accessibility = self.request_accessibility_permissions()?;
        let microphone = self.request_microphone_permissions()?;

        Ok(accessibility && microphone)
    }

    pub fn check_accessibility_permissions(&self) -> Result<bool, SystemError> {
        #[cfg(target_os = "macos")]
        {
            Ok(macos_accessibility_client::accessibility::application_is_trusted())
        }

        #[cfg(not(target_os = "macos"))]
        {
            Ok(true)
        }
    }

    pub fn request_accessibility_permissions(&self) -> Result<bool, SystemError> {
        #[cfg(target_os = "macos")]
        {
            Ok(macos_accessibility_client::accessibility::application_is_trusted_with_prompt())
        }

        #[cfg(not(target_os = "macos"))]
        {
            Ok(true)
        }
    }

    pub fn check_microphone_permissions(&self) -> Result<bool, SystemError> {
        match cpal::default_host().default_input_device() {
            Some(device) => match device.name() {
                Ok(_) => Ok(true),
                Err(_) => Ok(false),
            },
            None => Ok(false),
        }
    }

    pub fn request_microphone_permissions(&self) -> Result<bool, SystemError> {
        let host = cpal::default_host();

        match host.default_input_device() {
            Some(device) => {
                let config = match device.default_input_config() {
                    Ok(config) => config,
                    Err(_) => return Ok(false),
                };

                let stream_result = device.build_input_stream(
                    &config.into(),
                    |_data: &[f32], _: &cpal::InputCallbackInfo| {},
                    |err| eprintln!("Error in audio stream: {}", err),
                    None,
                );

                match stream_result {
                    Ok(_stream) => Ok(true),
                    Err(_) => Ok(false),
                }
            }
            None => Ok(false),
        }
    }

    #[cfg(target_os = "macos")]
    pub fn check_screen_recording_permissions(&self) -> Result<bool, SystemError> {
        use std::process::Command;

        let output = Command::new("sh")
            .arg("-c")
            .arg("osascript -e 'tell application \"System Events\" to get name of first window of first process'")
            .output();

        match output {
            Ok(output) => Ok(!output.stderr.len() > 0),
            Err(_) => Ok(false),
        }
    }

    #[cfg(target_os = "macos")]
    pub fn request_screen_recording_permissions(&self) -> Result<bool, SystemError> {
        use std::process::Command;

        if self.check_screen_recording_permissions()? {
            return Ok(true);
        }

        // If not, prompt the user to open System Preferences
        let _ = Command::new("sh")
            .arg("-c")
            .arg("osascript -e 'tell application \"System Preferences\" to activate' -e 'tell application \"System Preferences\" to reveal pane \"Security\" preference pane \"Privacy\"'")
            .output();

        Ok(false)
    }
}

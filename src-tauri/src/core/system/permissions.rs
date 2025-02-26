use crate::core::error::SystemError;
use cpal::{
    self,
    traits::{DeviceTrait, HostTrait},
};

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

pub fn check_microphone_permissions() -> Result<bool, SystemError> {
    // CPAL approach for checking microphone permissions
    // This will try to list input devices, which will trigger permission checks on platforms that need them
    match cpal::default_host().default_input_device() {
        Some(device) => {
            // Try to get the device name to ensure we have access
            match device.name() {
                Ok(_) => {
                    // Successfully got device name, which means we have permission
                    Ok(true)
                }
                Err(_) => {
                    // Failed to get device name, could be permission issue
                    Ok(false)
                }
            }
        }
        None => {
            // No input devices available or no permission
            Ok(false)
        }
    }
}

pub fn request_microphone_permissions() -> Result<bool, SystemError> {
    // On platforms like macOS, simply attempting to access the microphone
    // will trigger the permission dialog if needed
    let host = cpal::default_host();

    match host.default_input_device() {
        Some(device) => {
            // Try to create an input stream config
            let config = match device.default_input_config() {
                Ok(config) => config,
                Err(_) => return Ok(false),
            };

            // Try to build an input stream (this will prompt for permission if needed)
            // We're not actually going to use this stream, just build it to trigger permissions
            let stream_result = device.build_input_stream(
                &config.into(),
                |_data: &[f32], _: &cpal::InputCallbackInfo| {
                    // This callback won't actually run since we drop the stream immediately
                },
                |err| eprintln!("Error in audio stream: {}", err),
                None, // No timeout
            );

            // Check if we successfully built the stream
            match stream_result {
                Ok(_stream) => {
                    // Successfully built stream, we have permission
                    // We immediately drop the stream as we don't need it
                    Ok(true)
                }
                Err(_) => {
                    // Failed to build stream, could be permission denied
                    Ok(false)
                }
            }
        }
        None => {
            // No input device available
            Ok(false)
        }
    }
}

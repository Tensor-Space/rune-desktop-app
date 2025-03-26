use cpal::traits::{DeviceTrait, HostTrait};
use serde::{Deserialize, Serialize};

use crate::core::error::AudioError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDevice {
    pub name: String,
    pub id: String,
}

pub struct AudioDeviceService;

impl AudioDeviceService {
    pub fn new() -> Self {
        Self
    }

    pub fn list_devices(&self) -> Result<Vec<AudioDevice>, AudioError> {
        let host = cpal::default_host();
        Ok(host
            .input_devices()
            .map_err(|e| AudioError::Device(e.to_string()))?
            .filter_map(|device| {
                device.name().ok().map(|name| AudioDevice {
                    name: name.clone(),
                    id: name,
                })
            })
            .collect())
    }

    pub fn get_default_device(&self) -> Result<Option<AudioDevice>, AudioError> {
        let host = cpal::default_host();
        match host.default_input_device() {
            Some(device) => match device.name() {
                Ok(name) => Ok(Some(AudioDevice {
                    name: name.clone(),
                    id: name,
                })),
                Err(e) => Err(AudioError::Device(format!(
                    "Failed to get device name: {}",
                    e
                ))),
            },
            None => Ok(None),
        }
    }

    pub fn find_device_by_id(&self, id: &str) -> Result<Option<AudioDevice>, AudioError> {
        let devices = self.list_devices()?;
        Ok(devices.into_iter().find(|device| device.id == id))
    }

    pub fn find_device_by_name(&self, name: &str) -> Result<Option<AudioDevice>, AudioError> {
        let devices = self.list_devices()?;

        // Try exact match first
        let exact_match = devices.iter().find(|device| device.name == name);
        if exact_match.is_some() {
            return Ok(exact_match.cloned());
        }

        // Then try partial match
        Ok(devices
            .into_iter()
            .find(|device| device.name.contains(name)))
    }
}

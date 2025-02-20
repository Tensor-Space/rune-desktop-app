use cpal::traits::{DeviceTrait, HostTrait};
use serde::{Deserialize, Serialize};

use crate::core::error::AudioError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDevice {
    pub name: String,
    pub id: String,
}

pub struct AudioDevices;

impl AudioDevices {
    pub fn list() -> Result<Vec<AudioDevice>, AudioError> {
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
}

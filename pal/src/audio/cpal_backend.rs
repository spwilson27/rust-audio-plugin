use super::{AudioBackend, AudioStream, DeviceInfo, StreamConfig};
use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::Arc;

pub struct CpalAudioBackend {
    host: cpal::Host,
}

impl CpalAudioBackend {
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();
        Ok(Self { host })
    }
}

impl AudioBackend for CpalAudioBackend {
    fn enumerate_input_devices(&self) -> Vec<DeviceInfo> {
        self.host
            .input_devices()
            .map(|devices| {
                devices
                    .map(|d| {
                        #[allow(deprecated)]
                        let name = d.name().unwrap_or_else(|_| "Unknown Device".to_string());
                        DeviceInfo {
                            id: name.clone(),
                            name,
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn enumerate_output_devices(&self) -> Vec<DeviceInfo> {
        self.host
            .output_devices()
            .map(|devices| {
                devices
                    .map(|d| {
                        #[allow(deprecated)]
                        let name = d.name().unwrap_or_else(|_| "Unknown Device".to_string());
                        DeviceInfo {
                            id: name.clone(),
                            name,
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn create_input_stream(
        &self,
        device_id: &str,
        config: &StreamConfig,
        mut callback: Box<dyn FnMut(&[f32]) + Send>,
    ) -> Result<Box<dyn AudioStream>> {
        let device = self
            .host
            .input_devices()?
            .find(|d| {
                #[allow(deprecated)]
                d.name().is_ok_and(|n| n == device_id)
            })
            .context("Device not found")?;

        let cpal_config = cpal::StreamConfig {
            channels: config.channels,
            sample_rate: config.sample_rate,
            buffer_size: cpal::BufferSize::Fixed(config.buffer_size),
        };

        // TODO: Handle different sample formats (u16, i16, f32)
        // For now Assuming f32
        let stream = device.build_input_stream(
            &cpal_config,
            move |data: &[f32], _: &_| {
                callback(data);
            },
            move |err| {
                tracing::error!("Audio input stream error: {}", err);
            },
            None,
        )?;

        Ok(Box::new(CpalStream {
            stream: Arc::new(stream),
        }))
    }

    fn create_output_stream(
        &self,
        device_id: &str,
        config: &StreamConfig,
        mut callback: Box<dyn FnMut(&mut [f32]) + Send>,
    ) -> Result<Box<dyn AudioStream>> {
        let device = self
            .host
            .output_devices()?
            .find(|d| {
                #[allow(deprecated)]
                d.name().is_ok_and(|n| n == device_id)
            })
            .context("Device not found")?;

        let cpal_config = cpal::StreamConfig {
            channels: config.channels,
            sample_rate: config.sample_rate,
            buffer_size: cpal::BufferSize::Fixed(config.buffer_size),
        };

        let stream = device.build_output_stream(
            &cpal_config,
            move |data: &mut [f32], _: &_| {
                callback(data);
            },
            move |err| {
                tracing::error!("Audio output stream error: {}", err);
            },
            None,
        )?;

        Ok(Box::new(CpalStream {
            stream: Arc::new(stream),
        }))
    }
}

struct CpalStream {
    stream: Arc<cpal::Stream>,
}

unsafe impl Send for CpalStream {}

impl AudioStream for CpalStream {
    fn play(&self) -> Result<()> {
        self.stream.play().context("Failed to play stream")
    }

    fn pause(&self) -> Result<()> {
        self.stream.pause().context("Failed to pause stream")
    }
}

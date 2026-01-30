use anyhow::Result;

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub name: String,
    pub id: String, // Platform specific ID (can be name if unique, or internal index)
}

#[derive(Debug, Clone, Copy)]
pub struct StreamConfig {
    pub sample_rate: u32,
    pub buffer_size: u32,
    pub channels: u16,
}

pub trait AudioStream: Send {
    fn play(&self) -> Result<()>;
    fn pause(&self) -> Result<()>;
}

/// Platform Audio Backend trait.
/// Abstracting over cpal or other backends.
pub trait AudioBackend: Send + Sync {
    /// List available input devices
    fn enumerate_input_devices(&self) -> Vec<DeviceInfo>;

    /// List available output devices
    fn enumerate_output_devices(&self) -> Vec<DeviceInfo>;

    /// Create an input stream
    #[allow(clippy::type_complexity)]
    fn create_input_stream(
        &self,
        device_id: &str,
        config: &StreamConfig,
        callback: Box<dyn FnMut(&[f32]) + Send>,
    ) -> Result<Box<dyn AudioStream>>;

    /// Create an output stream
    #[allow(clippy::type_complexity)]
    fn create_output_stream(
        &self,
        device_id: &str,
        config: &StreamConfig,
        callback: Box<dyn FnMut(&mut [f32]) + Send>,
    ) -> Result<Box<dyn AudioStream>>;
}

// Re-export specific backends
#[cfg(feature = "cpal")]
pub mod cpal_backend;

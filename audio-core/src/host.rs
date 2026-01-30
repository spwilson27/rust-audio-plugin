use crate::context::Transport;

/// Abstraction for the host environment (DAW or Standalone App).
///
/// This trait allows the audio engine to query information about the host,
/// such as current sample rate, buffer size, and transport state.
pub trait AudioHost: Send + Sync {
    /// Get the current sample rate in Hz.
    fn get_sample_rate(&self) -> f64;

    /// Get the current maximum buffer size in frames.
    fn get_max_buffer_size(&self) -> u32;

    /// Get the current transport state (time info).
    fn get_transport(&self) -> Transport;
}

/// A no-op host implementation for testing.
pub struct NoOpHost;

impl AudioHost for NoOpHost {
    fn get_sample_rate(&self) -> f64 {
        44100.0
    }

    fn get_max_buffer_size(&self) -> u32 {
        512
    }

    fn get_transport(&self) -> Transport {
        Transport::default()
    }
}

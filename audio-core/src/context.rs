/// Configuration for the audio processing context.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProcessConfig {
    pub sample_rate: f64,
    pub max_buffer_size: usize,
    pub input_channels: usize,
    pub output_channels: usize,
}

impl Default for ProcessConfig {
    fn default() -> Self {
        Self {
            sample_rate: 44100.0,
            max_buffer_size: 512,
            input_channels: 2,
            output_channels: 2,
        }
    }
}

/// Transport state provided by the host.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transport {
    pub playing: bool,
    pub recording: bool,
    pub position_samples: i64,
    pub bpm: f64,
    pub time_sig_numerator: u8,
    pub time_sig_denominator: u8,
}

impl Default for Transport {
    fn default() -> Self {
        Self {
            playing: false,
            recording: false,
            position_samples: 0,
            bpm: 120.0,
            time_sig_numerator: 4,
            time_sig_denominator: 4,
        }
    }
}

/// Context passed to the process callback.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProcessContext<'a> {
    pub transport: &'a Transport,
    pub sample_rate: f64,
}

impl<'a> ProcessContext<'a> {
    pub fn new(transport: &'a Transport, sample_rate: f64) -> Self {
        Self {
            transport,
            sample_rate,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() {
        let config = ProcessConfig::default();
        assert_eq!(config.sample_rate, 44100.0);

        let transport = Transport::default();
        assert_eq!(transport.bpm, 120.0);
    }
}

use crate::buffer::AudioBufferMut;
use crate::context::{ProcessConfig, ProcessContext};
use crate::events::Event;

/// The core trait for audio processing components.
pub trait AudioProcessor: Send {
    /// Called when the processing parameters (sample rate, block size) change.
    fn prepare(&mut self, config: &ProcessConfig);

    /// Called to process a block of audio.
    /// Events for this block are provided in `events`.
    fn process(&mut self, buffer: &mut AudioBufferMut, context: &ProcessContext, events: &[Event]);

    /// Called to reset the internal state (e.g. clear delay lines).
    fn reset(&mut self);
}

/// A no-op processor for testing or placeholders.
pub struct NoOpProcessor;

impl AudioProcessor for NoOpProcessor {
    fn prepare(&mut self, _config: &ProcessConfig) {}
    fn process(
        &mut self,
        buffer: &mut AudioBufferMut,
        _context: &ProcessContext,
        _events: &[Event],
    ) {
        // Silence the buffer
        for channel in buffer.iter_mut() {
            channel.fill(0.0);
        }
    }
    fn reset(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::AudioBufferMut;
    use crate::context::Transport;

    #[test]
    fn test_noop_processor() {
        let mut processor = NoOpProcessor;
        let mut data = [vec![1.0; 4], vec![1.0; 4]];
        let mut slices: Vec<&mut [f32]> = data.iter_mut().map(|v| v.as_mut_slice()).collect();
        let mut buffer = AudioBufferMut::new(&mut slices);

        let transport = Transport::default();
        let context = ProcessContext::new(&transport, 44100.0);

        processor.process(&mut buffer, &context, &[]);

        for ch in buffer.iter_mut() {
            for sample in ch.iter() {
                assert_eq!(*sample, 0.0);
            }
        }
    }
}

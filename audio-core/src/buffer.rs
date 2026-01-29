use std::slice;

/// A safe wrapper around immutable non-interleaved audio data.
///
/// This struct allows access to audio samples as slices of slices.
/// It is designed to be zero-cost and easy to construct from raw pointers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioBuffer<'a> {
    // The outer slice holds references to each channel's data.
    // The inner slice holds the samples for that channel.
    channels: &'a [&'a [f32]],
}

impl<'a> AudioBuffer<'a> {
    /// Creates a new `AudioBuffer` from a slice of channel slices.
    #[inline]
    pub fn new(channels: &'a [&'a [f32]]) -> Self {
        Self { channels }
    }

    /// Returns the number of channels.
    #[inline]
    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }

    /// Returns the number of samples per channel (block size).
    /// Returns 0 if there are no channels.
    /// Assumes all channels have the same length.
    #[inline]
    pub fn samples(&self) -> usize {
        if self.channels.is_empty() {
            0
        } else {
            self.channels[0].len()
        }
    }

    /// returns the slice of channels
    #[inline]
    pub fn raw(&self) -> &'a [&'a [f32]] {
        self.channels
    }

    /// Returns a reference to the channel slice at the given index.
    #[inline]
    pub fn get_channel(&self, index: usize) -> Option<&'a [f32]> {
        self.channels.get(index).copied()
    }

    /// Iterates over the channels.
    #[inline]
    pub fn iter(&self) -> slice::Iter<'a, &'a [f32]> {
        self.channels.iter()
    }
}

/// A safe wrapper around mutable non-interleaved audio data.
#[derive(Debug, PartialEq)]
pub struct AudioBufferMut<'a> {
    channels: &'a mut [&'a mut [f32]],
}

impl<'a> AudioBufferMut<'a> {
    /// Creates a new `AudioBufferMut` from a mutable slice of mutable channel slices.
    #[inline]
    pub fn new(channels: &'a mut [&'a mut [f32]]) -> Self {
        Self { channels }
    }

    /// Returns the number of channels.
    #[inline]
    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }

    /// Returns the number of samples per channel.
    #[inline]
    pub fn samples(&self) -> usize {
        if self.channels.is_empty() {
            0
        } else {
            self.channels[0].len()
        }
    }

    /// returns the slice of channels
    #[inline]
    pub fn raw(&self) -> &[&'a mut [f32]] {
        self.channels
    }

    /// returns the mutable slice of channels
    #[inline]
    pub fn raw_mut(&mut self) -> &mut [&'a mut [f32]] {
        self.channels
    }

    /// Returns a reference to the channel slice at the given index.
    #[inline]
    pub fn get_channel(&self, index: usize) -> Option<&[f32]> {
        self.channels.get(index).map(|s| &**s)
    }

    /// Returns a mutable reference to the channel slice at the given index.
    #[inline]
    pub fn get_channel_mut(&mut self, index: usize) -> Option<&mut [f32]> {
        self.channels.get_mut(index).map(|s| &mut **s)
    }

    /// Iterates over the channels mutably.
    #[inline]
    pub fn iter_mut(&mut self) -> slice::IterMut<'_, &'a mut [f32]> {
        self.channels.iter_mut()
    }

    /// Copies data from a source buffer into this buffer.
    /// Panics if channel counts or lengths do not match.
    pub fn copy_from(&mut self, source: &AudioBuffer) {
        assert_eq!(self.channel_count(), source.channel_count());
        assert_eq!(self.samples(), source.samples());

        for (dst, src) in self.channels.iter_mut().zip(source.raw().iter()) {
            dst.copy_from_slice(src);
        }
    }

    /// Silences operations
    pub fn silence(&mut self) {
        for ch in self.channels.iter_mut() {
            ch.fill(0.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_buffer() {
        let channel1 = vec![1.0, 2.0, 3.0];
        let channel2 = vec![4.0, 5.0, 6.0];
        let channels = vec![channel1.as_slice(), channel2.as_slice()];

        // We need a stable slice reference
        let buf_ref = channels.as_slice();
        let buffer = AudioBuffer::new(buf_ref);

        assert_eq!(buffer.channel_count(), 2);
        assert_eq!(buffer.samples(), 3);
        assert_eq!(buffer.get_channel(0), Some(channel1.as_slice()));
        assert_eq!(buffer.get_channel(1), Some(channel2.as_slice()));
    }

    #[test]
    fn test_audio_buffer_mut() {
        let mut data1 = vec![0.0; 4];
        let mut data2 = vec![0.0; 4];

        {
            let mut channels = vec![data1.as_mut_slice(), data2.as_mut_slice()];
            let mut buffer = AudioBufferMut::new(channels.as_mut_slice());

            assert_eq!(buffer.channel_count(), 2);
            assert_eq!(buffer.samples(), 4);

            if let Some(ch) = buffer.get_channel_mut(0) {
                ch[0] = 1.0;
            }
        }

        assert_eq!(data1[0], 1.0);
    }

    #[test]
    fn test_copy_from() {
        let src_data1 = vec![1.0, 1.0];
        let src_data2 = vec![2.0, 2.0];
        let src_slices = vec![src_data1.as_slice(), src_data2.as_slice()];
        let src = AudioBuffer::new(&src_slices);

        let mut dst_data1 = vec![0.0, 0.0];
        let mut dst_data2 = vec![0.0, 0.0];
        {
            let mut dst_slices = vec![dst_data1.as_mut_slice(), dst_data2.as_mut_slice()];
            let mut dst = AudioBufferMut::new(&mut dst_slices);
            dst.copy_from(&src);
        }

        assert_eq!(dst_data1, src_data1);
        assert_eq!(dst_data2, src_data2);
    }
}

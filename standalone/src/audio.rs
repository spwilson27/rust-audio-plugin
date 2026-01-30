use anyhow::Result;
use audio_core::{AudioHost, AudioProcessor, Event, Transport};
use pal::audio::{AudioBackend, AudioStream, StreamConfig};
use std::sync::{Arc, Mutex, RwLock};

#[allow(dead_code)]
pub struct StandaloneAudioHost {
    backend: Box<dyn AudioBackend>,
    input_stream: Arc<Mutex<Option<Box<dyn AudioStream>>>>,
    output_stream: Arc<Mutex<Option<Box<dyn AudioStream>>>>,

    // Shared state
    sample_rate: Arc<RwLock<f64>>,
    buffer_size: Arc<RwLock<u32>>,

    // To protect processor from concurrent access if needed (though audio thread should be unique user of process())
    // But we need to share processor ownership or access.
    // Ideally, we move processor into the callback.
    // But we might need to access it from UI?
    // For now, we'll wrap it in a Mutex, but this violates "Real-Time Safety: No mutex locking on audio thread".
    // So we should ownership of processor to the audio thread/callback.
    // But how do we communicate/control parameters? Via Events/Atomic queues (rtrb/crossbeam).
    // The prompt says: "No allocations (Box, Vec, String) or mutex locking on the audio thread. Use rtrb or atomics."

    // For this MVP, we will use a naive Mutex but mark it as TODO to fix with atomics/lock-free.
    // OR we just put the processor inside the callback closure and don't touch it from outside except via messages.
    // We already have `Event` system in `audio-core`.
    // So we can send events to the processor via a queue.

    // So `StandaloneAudioHost` will NOT hold `AudioProcessor` directly after starting stream.
    // It will hold a sender to the processor queue.
    event_sender: crossbeam_channel::Sender<Event>,
}

// We need a wrapper to implement AudioHost that is Send/Sync and can be passed to Processor.
// But Processor is inside callback.
// The Processor `process` method takes `&ProcessContext`. `ProcessContext` wraps `Transport`.
// `StandaloneAudioHost` implements `AudioHost`, so we can pass a clone?
// `AudioHost` trait ref is usually passed to `process`? No, `prepare` takes config.
// `AudioProcessor` doesn't take `AudioHost` in `process`.
// `ProcessContext` in `audio-core` likely holds transport info.

impl StandaloneAudioHost {
    pub fn new() -> Result<Self> {
        // Initialize backend (CPAL)
        #[cfg(feature = "cpal")]
        let backend = Box::new(pal::audio::cpal_backend::CpalAudioBackend::new()?);

        #[cfg(not(feature = "cpal"))]
        let backend = anyhow::bail!("No audio backend enabled");

        Self::new_with_backend(backend)
    }

    pub fn new_with_backend(backend: Box<dyn AudioBackend>) -> Result<Self> {
        let (tx, _rx) = crossbeam_channel::unbounded(); // Placeholder

        Ok(Self {
            backend,
            input_stream: Arc::new(Mutex::new(None)),
            output_stream: Arc::new(Mutex::new(None)),
            sample_rate: Arc::new(RwLock::new(44100.0)),
            buffer_size: Arc::new(RwLock::new(512)),
            event_sender: tx,
        })
    }

    pub fn get_backend(&self) -> &dyn AudioBackend {
        &*self.backend
    }

    pub fn start_audio<P>(
        &mut self,
        device_id: &str,
        sample_rate: u32,
        buffer_size: u32,
        mut processor: P,
    ) -> Result<()>
    where
        P: AudioProcessor + 'static,
    {
        // Update state
        {
            let mut sr = self.sample_rate.write().unwrap();
            *sr = sample_rate as f64;
            let mut bs = self.buffer_size.write().unwrap();
            *bs = buffer_size;
        }

        let config = StreamConfig {
            sample_rate,
            buffer_size,
            channels: 2, // Stereo for now
        };

        // Prepare processor
        processor.prepare(&audio_core::context::ProcessConfig {
            sample_rate: sample_rate as f64,
            max_buffer_size: buffer_size as usize,
            input_channels: 2,
            output_channels: 2,
        });

        // Pre-allocate scratch buffers for planar audio
        // We assume stereo (2 channels) for now
        let mut left_buffer = vec![0.0f32; buffer_size as usize];
        let mut right_buffer = vec![0.0f32; buffer_size as usize];

        // Mock Transport
        let transport = Transport::default();
        // context creation moved inside callback

        let stream = self.backend.create_output_stream(
            device_id,
            &config,
            Box::new(move |output| {
                let frames = output.len() / 2;

                // Safety check
                if frames > left_buffer.len() {
                    tracing::error!(
                        "Frame count {} exceeds buffer size {}",
                        frames,
                        left_buffer.len()
                    );
                    // Fill silence and return
                    output.fill(0.0);
                    return;
                }

                // 1. Prepare Inputs (Silence for now as we don't have input stream)
                left_buffer[..frames].fill(0.0);
                right_buffer[..frames].fill(0.0);

                // 2. Wrap scratch buffers in AudioBufferMut
                {
                    // Create minimal scope for mutable borrow
                    let mut channel_slices =
                        [&mut left_buffer[..frames], &mut right_buffer[..frames]];
                    let mut buffer = audio_core::AudioBufferMut::new(&mut channel_slices);

                    // 3. Process
                    // Create per-block context
                    let context =
                        audio_core::context::ProcessContext::new(&transport, sample_rate as f64);
                    processor.process(&mut buffer, &context, &[]);
                }

                // 4. Interleave to Output
                // output is [L, R, L, R...]
                let mut out_idx = 0;
                for i in 0..frames {
                    if out_idx + 1 < output.len() {
                        output[out_idx] = left_buffer[i];
                        output[out_idx + 1] = right_buffer[i];
                        out_idx += 2;
                    }
                }
            }),
        )?;

        stream.play()?;
        *self.output_stream.lock().unwrap() = Some(stream);

        Ok(())
    }
}

impl AudioHost for StandaloneAudioHost {
    fn get_sample_rate(&self) -> f64 {
        *self.sample_rate.read().unwrap()
    }

    fn get_max_buffer_size(&self) -> u32 {
        *self.buffer_size.read().unwrap()
    }

    fn get_transport(&self) -> Transport {
        Transport::default()
    }
}

/// An event that occurs during audio processing.
/// Events are sample-accurate (associated with a delta frame).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    NoteOn(NoteOnEvent),
    NoteOff(NoteOffEvent),
    ParamChange(ParamChangeEvent),
    Midi(MidiEvent),
}

impl Event {
    pub fn delta_frames(&self) -> u32 {
        match self {
            Event::NoteOn(e) => e.delta_frames,
            Event::NoteOff(e) => e.delta_frames,
            Event::ParamChange(e) => e.delta_frames,
            Event::Midi(e) => e.delta_frames,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NoteOnEvent {
    pub delta_frames: u32,
    pub note: u8,
    pub velocity: f32, // 0.0 - 1.0
    pub channel: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NoteOffEvent {
    pub delta_frames: u32,
    pub note: u8,
    pub velocity: f32, // 0.0 - 1.0 usually 0
    pub channel: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParamChangeEvent {
    pub delta_frames: u32,
    pub param_id: u32,
    pub value: f32, // normalized 0.0 - 1.0
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MidiEvent {
    pub delta_frames: u32,
    pub data: [u8; 3],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_sizes() {
        // Ensure events are small enough to be copied cheaply
        use std::mem::size_of;
        assert!(size_of::<Event>() <= 32);
    }

    #[test]
    fn test_delta_frames() {
        let evt = Event::NoteOn(NoteOnEvent {
            delta_frames: 10,
            note: 60,
            velocity: 0.8,
            channel: 1,
        });

        assert_eq!(evt.delta_frames(), 10);
    }
}

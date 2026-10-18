use super::{
    note::{Note, Rest},
    time_signature::TimeSignature,
};

#[derive(Clone, PartialEq)]
pub struct Measure {
    /// Where the measure is within the score, measured in ticks since the beginning
    pub timestamp: u32,
    pub notes: Vec<Note>,
    pub rests: Vec<Rest>,
    pub time_signature: TimeSignature,
    /// A repeated passage starts at this measure
    pub repeat_start: bool,
    /// How many times the repeated passage is played in total, when one ends
    /// at this measure
    pub repeat_end: Option<u32>,
}

impl Measure {
    /// Get the length of the measure in ticks
    pub fn get_length(&self) -> u32 {
        let ticks_per_note = self.time_signature.note_value.get_ticks();
        ticks_per_note * self.time_signature.notes_per_measure
    }

    pub fn contains_timestamp(&self, timestamp: u32) -> bool {
        let next_measure_start = self.timestamp + self.get_length();

        self.timestamp <= timestamp && timestamp < next_measure_start
    }

    pub fn end_timestamp(&self) -> u32 {
        self.timestamp + self.get_length()
    }
}

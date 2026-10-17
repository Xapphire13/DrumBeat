use super::{note::Note, time_signature::TimeSignature};

#[derive(Clone, PartialEq)]
pub struct Measure {
    /// Where the measure is within the score, measured in ticks since the beginning
    pub timestamp: u32,
    pub notes: Vec<Note>,
    pub time_signature: TimeSignature,
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
}

use super::{note::Note, time_signature::TimeSignature};

#[derive(Clone, PartialEq)]
pub struct Measure {
    /// Where the measure is within the score, measured by the number of 256th notes since the beginning
    pub timestamp: u32,
    pub notes: Vec<Note>,
    pub time_signature: TimeSignature,
}

impl Measure {
    pub fn add_note(&mut self, note: Note) {
        self.notes.push(note);
        self.notes.sort_by_key(|note| note.timestamp);
    }

    pub fn remove_note(&mut self, note: Note) {
        self.notes
            .retain(|n| n.surface != note.surface || n.timestamp != note.timestamp);
    }

    /// Get the length of the measure in 256th note ticks
    pub fn get_length(&self) -> u32 {
        let ticks_per_note = self.time_signature.note_value.get_ticks();
        ticks_per_note * self.time_signature.notes_per_measure
    }

    /// Size of the measure in 256th note ticks
    pub fn get_size(&self) -> u32 {
        self.time_signature.notes_per_measure * self.time_signature.note_value.get_ticks()
    }

    pub fn contains_timestamp(&self, timestamp: u32) -> bool {
        let next_measure_start = self.timestamp + self.get_size();

        self.timestamp <= timestamp && timestamp < next_measure_start
    }
}

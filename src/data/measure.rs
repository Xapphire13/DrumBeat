use super::{
    note::{GraceNote, Note, Rest},
    time_signature::TimeSignature,
};

#[derive(Clone, PartialEq)]
pub struct Measure {
    /// Where the measure is within the score, measured in ticks since the beginning
    pub timestamp: u32,
    pub notes: Vec<Note>,
    pub rests: Vec<Rest>,
    /// Sorted by timestamp
    pub grace_notes: Vec<GraceNote>,
    pub time_signature: TimeSignature,
    /// A repeated passage starts at this measure
    pub repeat_start: bool,
    /// How many times the repeated passage is played in total, when one ends
    /// at this measure
    pub repeat_end: Option<u32>,
    /// Whether the measure's beats add up to its time signature
    pub fill: Fill,
}

/// How a measure's beats compare with its time signature. Scores that are
/// still being written often have bars with too few or too many beats
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Fill {
    /// Every voice fills the measure exactly
    Full,
    /// A voice ends early. The measure keeps its written length, so the rest
    /// of it is silent
    Short,
    /// A voice runs past the end. The measure is lengthened to `length` ticks
    /// to fit it, so nothing overlaps the next measure
    Over { length: u32 },
}

impl Measure {
    /// Get the length of the measure in ticks
    pub fn get_length(&self) -> u32 {
        match self.fill {
            Fill::Over { length } => length,
            Fill::Full | Fill::Short => self.written_length(),
        }
    }

    /// The length the time signature gives the measure, in ticks
    pub fn written_length(&self) -> u32 {
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

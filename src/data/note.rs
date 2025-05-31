#[derive(Clone, PartialEq, Debug)]
pub enum NoteValue {
    Quarter,
    Eighth,
    Sixteenth,
    ThirtySecond,
}

impl NoteValue {
    /// Get the number of 256th note ticks in this note value
    pub fn get_ticks(&self) -> u32 {
        match self {
            NoteValue::Quarter => 64,
            NoteValue::Eighth => 32,
            NoteValue::Sixteenth => 16,
            NoteValue::ThirtySecond => 8,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum Surface {
    Snare,
    Hihat,
    Tom1,
    Tom2,
    FloorTom,
    Crash,
    Ride,
    Kick,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Note {
    pub value: NoteValue,
    pub surface: Surface,
    /** Where the note is within the score, measured by the number of 256th notes since the beginning */
    pub timestamp: u32,
}

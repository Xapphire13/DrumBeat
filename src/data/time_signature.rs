use super::note::NoteValue;

#[derive(Clone, PartialEq)]
pub struct TimeSignature {
    /// The number of notes per measure. This represents the top number
    /// in the time signature
    pub notes_per_measure: u32,
    /// The note value for the measure. This represents the bottom number
    /// in the time signature
    pub note_value: NoteValue,
}

impl TimeSignature {
    /// As it's written, e.g. "6/8"
    pub fn label(&self) -> String {
        let denominator = match self.note_value {
            NoteValue::Whole => 1,
            NoteValue::Half => 2,
            NoteValue::Quarter => 4,
            NoteValue::Eighth => 8,
            NoteValue::Sixteenth => 16,
            NoteValue::ThirtySecond => 32,
            NoteValue::SixtyFourth => 64,
        };
        format!("{}/{denominator}", self.notes_per_measure)
    }
}

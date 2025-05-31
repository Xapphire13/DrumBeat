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

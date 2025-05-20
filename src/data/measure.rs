use super::note::Note;

#[derive(Clone)]
pub struct Measure {
    /** Where the measure is within the score, measured by the number of 256th notes since the beginning */
    pub timestamp: u32,
    pub notes: Vec<Note>,
}

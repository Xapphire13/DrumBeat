use super::note::Note;

#[derive(Clone, PartialEq)]
pub struct Measure {
    /// Where the measure is within the score, measured by the number of 256th notes since the beginning
    pub timestamp: u32,
    /// Length of the measure in 256th notes
    pub length: u32,
    pub notes: Vec<Note>,
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
}

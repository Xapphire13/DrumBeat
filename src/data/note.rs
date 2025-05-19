#[derive(Clone, PartialEq)]
pub enum NoteValue {
    Quarter,
    Eighth,
    Sixteenth,
    Thirtysecondth,
}

#[derive(Clone, PartialEq)]
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

#[derive(Clone, PartialEq)]
pub struct Note {
    pub value: NoteValue,
    pub surface: Surface,
    /** Where the note is within the score, measured by the number of 256th notes since the beginning */
    pub timestamp: u32,
}

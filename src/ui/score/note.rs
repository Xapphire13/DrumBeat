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
    Highhat,
    Tom1,
    Tom2,
    FloorTom,
    Crash,
    Ride,
}

#[derive(Clone, PartialEq)]
pub struct Note {
    pub value: NoteValue,
    pub surface: Surface,
    pub offset: u8,
}

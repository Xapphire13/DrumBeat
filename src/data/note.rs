use super::drum_piece::DrumPiece;

/// Number of ticks in a quarter note. Divisible by 2, 3, 5 and their powers
/// up to 64th notes, so dotted notes and common tuplets land on whole ticks
pub const TICKS_PER_QUARTER: u32 = 960;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum NoteValue {
    Whole,
    Half,
    Quarter,
    Eighth,
    Sixteenth,
    ThirtySecond,
    SixtyFourth,
}

impl NoteValue {
    /// Get the number of ticks in this note value
    pub fn get_ticks(&self) -> u32 {
        match self {
            NoteValue::Whole => TICKS_PER_QUARTER * 4,
            NoteValue::Half => TICKS_PER_QUARTER * 2,
            NoteValue::Quarter => TICKS_PER_QUARTER,
            NoteValue::Eighth => TICKS_PER_QUARTER / 2,
            NoteValue::Sixteenth => TICKS_PER_QUARTER / 4,
            NoteValue::ThirtySecond => TICKS_PER_QUARTER / 8,
            NoteValue::SixtyFourth => TICKS_PER_QUARTER / 16,
        }
    }
}

/// A tuplet ratio, e.g. a triplet is 3 notes in the time of 2
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Tuplet {
    pub notes: u32,
    pub in_time_of: u32,
}

impl Tuplet {
    pub const TRIPLET: Tuplet = Tuplet {
        notes: 3,
        in_time_of: 2,
    };
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Duration {
    pub value: NoteValue,
    pub dots: u8,
    pub tuplet: Option<Tuplet>,
}

impl Duration {
    pub fn new(value: NoteValue) -> Self {
        Self {
            value,
            dots: 0,
            tuplet: None,
        }
    }

    /// Get the length of this duration in ticks
    pub fn get_ticks(&self) -> u32 {
        let base = self.value.get_ticks();
        // Each dot adds half of the previous addition: base * (2 - 1/2^dots)
        let dotted = base * 2 - (base >> self.dots);

        match self.tuplet {
            Some(Tuplet { notes, in_time_of }) => dotted * in_time_of / notes,
            None => dotted,
        }
    }
}

/// How a note is played, where that changes how it is drawn
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Articulation {
    Normal,
    HihatClosed,
    HihatHalfOpen,
    HihatOpen,
    RideBell,
}

impl Articulation {
    /// Work out the articulation from the General MIDI note, and Guitar Pro's
    /// name for the sound when there is one, e.g. "Hi-Hat (open)"
    pub fn new(midi_note: u8, name: Option<&str>) -> Self {
        let name = name.map(str::to_lowercase).unwrap_or_default();
        if name.contains("bell") || (name.is_empty() && midi_note == 53) {
            Articulation::RideBell
        } else if name.contains("hi-hat") && name.contains("half") {
            Articulation::HihatHalfOpen
        } else if (name.contains("hi-hat") && name.contains("open"))
            || (name.is_empty() && midi_note == 46)
        {
            Articulation::HihatOpen
        } else if (name.contains("hi-hat") && name.contains("closed"))
            || (name.is_empty() && midi_note == 42)
        {
            Articulation::HihatClosed
        } else {
            Articulation::Normal
        }
    }

    pub fn is_open_hihat(self) -> bool {
        matches!(self, Articulation::HihatOpen | Articulation::HihatHalfOpen)
    }
}

/// A sign drawn over a note
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Annotation {
    /// "o": the hi-hat is played open (the pedal lifts)
    Open,
    /// A circle with a slash: the hi-hat is played half open
    HalfOpen,
    /// "+": the hi-hat closes again after being open
    Closed,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Rest {
    pub duration: Duration,
    /// Where the rest is within the score, measured in ticks since the beginning
    pub timestamp: u32,
    pub voice: u8,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Note {
    pub duration: Duration,
    /// The General MIDI percussion note number, kept so the exact
    /// articulation (e.g. open vs closed hi-hat) is not lost
    pub midi_note: u8,
    /// Where the note is within the score, measured in ticks since the beginning
    pub timestamp: u32,
    /// True when this note continues a tied note, so it should not be played
    pub tied: bool,
    /// Which voice the note is in. Voice 0 is drawn with stems up, the others
    /// (usually the feet) with stems down
    pub voice: u8,
    pub articulation: Articulation,
    pub annotation: Option<Annotation>,
}

impl Note {
    /// The physical drum piece this note is played on, if the MIDI note is a known one
    pub fn drum_piece(&self) -> Option<DrumPiece> {
        DrumPiece::from_gm_note(self.midi_note)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_durations() {
        assert_eq!(Duration::new(NoteValue::Quarter).get_ticks(), 960);
        assert_eq!(Duration::new(NoteValue::Sixteenth).get_ticks(), 240);
    }

    #[test]
    fn dotted_durations() {
        let mut dotted_quarter = Duration::new(NoteValue::Quarter);
        dotted_quarter.dots = 1;
        assert_eq!(dotted_quarter.get_ticks(), 1440);

        dotted_quarter.dots = 2;
        assert_eq!(dotted_quarter.get_ticks(), 1680);
    }

    #[test]
    fn tuplet_durations() {
        let mut eighth_triplet = Duration::new(NoteValue::Eighth);
        eighth_triplet.tuplet = Some(Tuplet::TRIPLET);
        assert_eq!(eighth_triplet.get_ticks(), 320);

        let mut quintuplet = Duration::new(NoteValue::Sixteenth);
        quintuplet.tuplet = Some(Tuplet {
            notes: 5,
            in_time_of: 4,
        });
        assert_eq!(quintuplet.get_ticks(), 192);
    }
}

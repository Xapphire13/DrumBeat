use super::{drum_piece::DrumPiece, drum_voice::DrumVoice};

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

/// A sign drawn over a note
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Annotation {
    /// "o": the hi-hat is played open (the pedal lifts)
    Open,
    /// A circle with a slash: the hi-hat is played half open
    HalfOpen,
    /// "+": the hi-hat closes again after being open
    Closed,
    /// An apostrophe: the cymbal is grabbed straight after it's hit
    Choke,
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
    /// Guitar Pro's number for the sound, e.g. 91 for a rim shot. See
    /// `DrumVoice::from_gp_note`
    pub midi_note: u8,
    /// Where the note is within the score, measured in ticks since the beginning
    pub timestamp: u32,
    /// True when this note continues a tied note, so it should not be played
    pub tied: bool,
    /// Which voice the note is in. Voice 0 is drawn with stems up, the others
    /// (usually the feet) with stems down
    pub voice: u8,
    pub annotation: Option<Annotation>,
}

/// How far ahead of the next note each grace note is played: a 64th note
/// (about 30 ms at 120 BPM), or less when the previous note is closer
pub const GRACE_NOTE_LEAD: u32 = TICKS_PER_QUARTER / 16;

/// A short note played just before a main note, taking no time in the bar,
/// e.g. the soft stroke of a flam or the two of a drag
#[derive(Clone, PartialEq, Debug)]
pub struct GraceNote {
    /// Guitar Pro's number for the sound, as for `Note::midi_note`
    pub midi_note: u8,
    /// When the grace note is played, in ticks since the beginning. Slightly
    /// before `main_timestamp`
    pub timestamp: u32,
    /// Where the main note it leads into is written
    pub main_timestamp: u32,
    /// How many grace notes, counting this one, are between it and the main
    /// note: 1 for the one right before it
    pub steps_before_main: u8,
    pub voice: u8,
}

impl GraceNote {
    pub fn drum_voice(&self) -> Option<DrumVoice> {
        DrumVoice::from_gp_note(self.midi_note)
    }
}

impl Note {
    /// The sound this note is played with, if it's one of the kit's
    pub fn drum_voice(&self) -> Option<DrumVoice> {
        DrumVoice::from_gp_note(self.midi_note)
    }

    /// The physical drum piece this note is played on, if it's one of the kit's
    pub fn drum_piece(&self) -> Option<DrumPiece> {
        self.drum_voice().map(DrumVoice::piece)
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

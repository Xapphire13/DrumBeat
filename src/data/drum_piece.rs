/// A physical piece of the drum kit. Several MIDI notes can map to the same
/// piece, e.g. open, closed and pedal hi-hat are all played on the hi-hat
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum DrumPiece {
    Kick,
    Snare,
    Hihat,
    HihatPedal,
    Tom1,
    Tom2,
    FloorTom,
    Crash,
    Ride,
    China,
    Splash,
}

impl DrumPiece {
    /// Look up the drum piece for a General MIDI percussion note number
    pub fn from_gm_note(midi_note: u8) -> Option<DrumPiece> {
        match midi_note {
            35 | 36 => Some(DrumPiece::Kick),
            // Side stick, acoustic snare, electric snare
            37 | 38 | 40 => Some(DrumPiece::Snare),
            // Closed and open hi-hat
            42 | 46 => Some(DrumPiece::Hihat),
            44 => Some(DrumPiece::HihatPedal),
            // High tom, hi-mid tom
            48 | 50 => Some(DrumPiece::Tom1),
            // Low tom, low-mid tom
            45 | 47 => Some(DrumPiece::Tom2),
            // Low floor tom, high floor tom
            41 | 43 => Some(DrumPiece::FloorTom),
            49 | 57 => Some(DrumPiece::Crash),
            // Ride cymbal 1, ride bell, ride cymbal 2
            51 | 53 | 59 => Some(DrumPiece::Ride),
            52 => Some(DrumPiece::China),
            55 => Some(DrumPiece::Splash),
            _ => None,
        }
    }

    /// Whether the piece is played with the feet, so it is drawn with its
    /// stem down
    pub fn is_played_with_feet(self) -> bool {
        matches!(self, DrumPiece::Kick | DrumPiece::HihatPedal)
    }

    /// Whether the piece is drawn with an X note head
    pub fn is_cymbal(self) -> bool {
        matches!(
            self,
            DrumPiece::Hihat
                | DrumPiece::HihatPedal
                | DrumPiece::Crash
                | DrumPiece::Ride
                | DrumPiece::China
                | DrumPiece::Splash
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn several_notes_map_to_one_piece() {
        assert_eq!(DrumPiece::from_gm_note(42), Some(DrumPiece::Hihat));
        assert_eq!(DrumPiece::from_gm_note(46), Some(DrumPiece::Hihat));
        assert_eq!(DrumPiece::from_gm_note(35), Some(DrumPiece::Kick));
        assert_eq!(DrumPiece::from_gm_note(36), Some(DrumPiece::Kick));
    }

    #[test]
    fn unknown_notes_have_no_piece() {
        assert_eq!(DrumPiece::from_gm_note(39), None);
        assert_eq!(DrumPiece::from_gm_note(0), None);
    }
}

/// A physical piece of the drum kit. Several voices can be played on the same
/// piece, e.g. open, closed and half-open hi-hat are all played on the hi-hat
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum DrumPiece {
    Kick,
    /// The second pedal of a double kick
    Kick2,
    Snare,
    Hihat,
    HihatPedal,
    HighTom,
    MidTom,
    LowTom,
    VeryLowTom,
    HighFloorTom,
    LowFloorTom,
    Crash,
    Ride,
    China,
    Splash,
    Cowbell,
}

impl DrumPiece {
    /// Whether the piece is played with the feet, so it is drawn with its
    /// stem down
    pub fn is_played_with_feet(self) -> bool {
        matches!(
            self,
            DrumPiece::Kick | DrumPiece::Kick2 | DrumPiece::HihatPedal
        )
    }
}

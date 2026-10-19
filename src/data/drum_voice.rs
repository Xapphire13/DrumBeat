use super::drum_piece::DrumPiece;

/// One sound of the kit, e.g. the ride's bell or a choked crash. Guitar Pro
/// gives each its own note number, which follows General MIDI for the common
/// sounds and adds its own for the rest. The toms are named for a kit with a
/// high, mid and floor tom, which Guitar Pro numbers 50, 48 and 45
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum DrumVoice {
    Kick,
    /// The second pedal of a double kick
    Kick2,
    Snare,
    SnareElectric,
    RimShot,
    SideStick,
    HihatClosed,
    HihatHalfOpen,
    HihatOpen,
    HihatPedal,
    HighTom,
    MidTom,
    LowTom,
    VeryLowTom,
    FloorTom,
    LowFloorTom,
    RideEdge,
    RideMiddle,
    RideBell,
    RideChoke,
    CrashHigh,
    CrashMedium,
    CrashHighChoke,
    CrashMediumChoke,
    China,
    ChinaChoke,
    Splash,
    SplashChoke,
    CowbellLow,
    CowbellLowTip,
    CowbellMedium,
    CowbellMediumTip,
    CowbellHigh,
    CowbellHighTip,
}

/// The shape of a note's head
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NoteHead {
    Normal,
    X,
    /// Hollow, for the ride's bell
    Diamond,
    /// For the cowbell
    Triangle,
}

impl DrumVoice {
    /// The voice Guitar Pro plays on `note`, if it's one of the kit's. Hand
    /// percussion and the metronome aren't
    pub fn from_gp_note(note: u8) -> Option<DrumVoice> {
        use DrumVoice::*;
        Some(match note {
            35 => Kick2,
            36 => Kick,
            38 => Snare,
            40 => SnareElectric,
            91 => RimShot,
            31 | 37 => SideStick,
            42 => HihatClosed,
            92 => HihatHalfOpen,
            46 => HihatOpen,
            44 => HihatPedal,
            50 => HighTom,
            48 => MidTom,
            47 => LowTom,
            45 => FloorTom,
            43 => VeryLowTom,
            41 => LowFloorTom,
            59 | 93 => RideEdge,
            51 | 126 => RideMiddle,
            53 | 127 => RideBell,
            29 | 94 => RideChoke,
            49 => CrashHigh,
            57 => CrashMedium,
            97 => CrashHighChoke,
            98 => CrashMediumChoke,
            52 => China,
            96 => ChinaChoke,
            55 => Splash,
            95 => SplashChoke,
            99 => CowbellLow,
            100 => CowbellLowTip,
            56 => CowbellMedium,
            101 => CowbellMediumTip,
            102 => CowbellHigh,
            103 => CowbellHighTip,
            _ => return None,
        })
    }

    /// The piece of the kit it's played on
    pub fn piece(self) -> DrumPiece {
        use DrumVoice::*;
        match self {
            Kick => DrumPiece::Kick,
            Kick2 => DrumPiece::Kick2,
            Snare | SnareElectric | RimShot | SideStick => DrumPiece::Snare,
            HihatClosed | HihatHalfOpen | HihatOpen => DrumPiece::Hihat,
            HihatPedal => DrumPiece::HihatPedal,
            HighTom => DrumPiece::HighTom,
            MidTom => DrumPiece::MidTom,
            LowTom => DrumPiece::LowTom,
            VeryLowTom => DrumPiece::VeryLowTom,
            FloorTom => DrumPiece::FloorTom,
            LowFloorTom => DrumPiece::LowFloorTom,
            RideEdge | RideMiddle | RideBell | RideChoke => DrumPiece::Ride,
            CrashHigh | CrashMedium | CrashHighChoke | CrashMediumChoke => DrumPiece::Crash,
            China | ChinaChoke => DrumPiece::China,
            Splash | SplashChoke => DrumPiece::Splash,
            CowbellLow | CowbellLowTip | CowbellMedium | CowbellMediumTip | CowbellHigh
            | CowbellHighTip => DrumPiece::Cowbell,
        }
    }

    /// What it's called, as Guitar Pro names it
    pub fn name(self) -> &'static str {
        use DrumVoice::*;
        match self {
            Kick => "Kick",
            Kick2 => "Kick 2",
            Snare => "Snare",
            SnareElectric => "Snare (electric)",
            RimShot => "Snare (rim shot)",
            SideStick => "Snare (side stick)",
            HihatClosed => "Hi-hat (closed)",
            HihatHalfOpen => "Hi-hat (half open)",
            HihatOpen => "Hi-hat (open)",
            HihatPedal => "Hi-hat (pedal)",
            HighTom => "High tom",
            MidTom => "Mid tom",
            LowTom => "Low tom",
            VeryLowTom => "Very low tom",
            FloorTom => "Floor tom",
            LowFloorTom => "Low floor tom",
            RideEdge => "Ride (edge)",
            RideMiddle => "Ride (middle)",
            RideBell => "Ride (bell)",
            RideChoke => "Ride (choke)",
            CrashHigh => "Crash high",
            CrashMedium => "Crash medium",
            CrashHighChoke => "Crash high (choke)",
            CrashMediumChoke => "Crash medium (choke)",
            China => "China",
            ChinaChoke => "China (choke)",
            Splash => "Splash",
            SplashChoke => "Splash (choke)",
            CowbellLow => "Cowbell low",
            CowbellLowTip => "Cowbell low (tip)",
            CowbellMedium => "Cowbell medium",
            CowbellMediumTip => "Cowbell medium (tip)",
            CowbellHigh => "Cowbell high",
            CowbellHighTip => "Cowbell high (tip)",
        }
    }

    /// The General MIDI note it's sent as. Sounds General MIDI doesn't have
    /// are sent as the nearest one it does
    pub fn gm_note(self) -> u8 {
        use DrumVoice::*;
        match self {
            Kick => 36,
            Kick2 => 35,
            Snare | RimShot => 38,
            SnareElectric => 40,
            SideStick => 37,
            HihatClosed => 42,
            HihatHalfOpen | HihatOpen => 46,
            HihatPedal => 44,
            HighTom => 50,
            MidTom => 48,
            LowTom => 47,
            FloorTom => 45,
            VeryLowTom => 43,
            LowFloorTom => 41,
            RideEdge => 59,
            RideMiddle | RideChoke => 51,
            RideBell => 53,
            CrashHigh | CrashHighChoke => 49,
            CrashMedium | CrashMediumChoke => 57,
            China | ChinaChoke => 52,
            Splash | SplashChoke => 55,
            CowbellLow | CowbellLowTip | CowbellMedium | CowbellMediumTip | CowbellHigh
            | CowbellHighTip => 56,
        }
    }

    pub fn head(self) -> NoteHead {
        use DrumVoice::*;
        match self {
            RideBell => NoteHead::Diamond,
            CowbellLow | CowbellLowTip | CowbellMedium | CowbellMediumTip | CowbellHigh
            | CowbellHighTip => NoteHead::Triangle,
            SideStick | HihatClosed | HihatHalfOpen | HihatOpen | HihatPedal | RideEdge
            | RideMiddle | RideChoke | CrashHigh | CrashMedium | CrashHighChoke
            | CrashMediumChoke | China | ChinaChoke | Splash | SplashChoke => NoteHead::X,
            Kick | Kick2 | Snare | SnareElectric | RimShot | HighTom | MidTom | LowTom
            | VeryLowTom | FloorTom | LowFloorTom => NoteHead::Normal,
        }
    }

    /// A cymbal grabbed straight after it's hit, so it stops ringing
    pub fn is_choke(self) -> bool {
        use DrumVoice::*;
        matches!(
            self,
            RideChoke | CrashHighChoke | CrashMediumChoke | ChinaChoke | SplashChoke
        )
    }

    pub fn is_open_hihat(self) -> bool {
        matches!(self, DrumVoice::HihatOpen | DrumVoice::HihatHalfOpen)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guitar_pro_numbers_pick_the_voice() {
        assert_eq!(DrumVoice::from_gp_note(38), Some(DrumVoice::Snare));
        assert_eq!(DrumVoice::from_gp_note(91), Some(DrumVoice::RimShot));
        assert_eq!(DrumVoice::from_gp_note(92), Some(DrumVoice::HihatHalfOpen));
        // Guitar Pro has two numbers for some sounds
        assert_eq!(DrumVoice::from_gp_note(53), Some(DrumVoice::RideBell));
        assert_eq!(DrumVoice::from_gp_note(127), Some(DrumVoice::RideBell));
    }

    #[test]
    fn hand_percussion_has_no_voice() {
        // Hand clap, tambourine, metronome
        assert_eq!(DrumVoice::from_gp_note(39), None);
        assert_eq!(DrumVoice::from_gp_note(54), None);
        assert_eq!(DrumVoice::from_gp_note(33), None);
    }

    #[test]
    fn every_voice_is_sent_as_general_midi() {
        for note in 0..=127 {
            if let Some(voice) = DrumVoice::from_gp_note(note) {
                assert!((35..=59).contains(&voice.gm_note()), "{voice:?}");
                // Notes General MIDI already has are sent as they are
                if note <= 59 && note != 31 && note != 29 {
                    assert_eq!(voice.gm_note(), note, "{voice:?}");
                }
            }
        }
    }
}

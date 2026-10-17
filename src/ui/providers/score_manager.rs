use dioxus::prelude::*;

use crate::data::{
    measure::Measure,
    note::{Duration, Note, NoteValue},
    score::Score,
    tempo::TempoMap,
    time_signature::TimeSignature,
};

const KICK: u8 = 36;
const SNARE: u8 = 38;
const CLOSED_HIHAT: u8 = 42;

fn note(value: NoteValue, midi_note: u8, timestamp: u32) -> Note {
    Note {
        duration: Duration::new(value),
        midi_note,
        timestamp,
        tied: false,
    }
}

pub fn use_score_manager_provider() -> Signal<Score> {
    use_context_provider(|| {
        Signal::new(Score {
            tempo_map: TempoMap::new(120.0),
            measures: vec![
                Measure {
                    timestamp: 0,
                    time_signature: TimeSignature {
                        notes_per_measure: 4,
                        note_value: NoteValue::Quarter,
                    },
                    notes: vec![
                        note(NoteValue::Quarter, KICK, 0),
                        note(NoteValue::Quarter, CLOSED_HIHAT, 0),
                        note(NoteValue::Eighth, CLOSED_HIHAT, 480),
                        note(NoteValue::Quarter, SNARE, 960),
                        note(NoteValue::Quarter, CLOSED_HIHAT, 960),
                        note(NoteValue::Eighth, CLOSED_HIHAT, 1440),
                        note(NoteValue::Quarter, KICK, 1920),
                        note(NoteValue::Quarter, CLOSED_HIHAT, 1920),
                        note(NoteValue::Eighth, CLOSED_HIHAT, 2400),
                        note(NoteValue::Quarter, SNARE, 2880),
                        note(NoteValue::Quarter, CLOSED_HIHAT, 2880),
                        note(NoteValue::Eighth, CLOSED_HIHAT, 3360),
                    ],
                },
                Measure {
                    timestamp: 3840,
                    time_signature: TimeSignature {
                        notes_per_measure: 4,
                        note_value: NoteValue::Quarter,
                    },
                    notes: vec![
                        note(NoteValue::Quarter, SNARE, 3840),
                        note(NoteValue::Quarter, SNARE, 4800),
                        note(NoteValue::Quarter, SNARE, 5760),
                        note(NoteValue::Quarter, SNARE, 6720),
                    ],
                },
            ],
        })
    })
}

pub fn use_score_manager() -> Signal<Score> {
    use_context::<Signal<Score>>()
}

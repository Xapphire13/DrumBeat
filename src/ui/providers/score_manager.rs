use dioxus::prelude::*;

use crate::data::{
    measure::Measure,
    note::{Note, NoteValue, Surface},
    score::Score,
    time_signature::TimeSignature,
};

pub fn use_score_manager_provider() -> Signal<Score> {
    use_context_provider(|| {
        Signal::new(Score {
            measures: vec![
                Measure {
                    timestamp: 0,
                    time_signature: TimeSignature {
                        notes_per_measure: 4,
                        note_value: NoteValue::Quarter,
                    },
                    notes: vec![
                        Note {
                            value: NoteValue::Quarter,
                            surface: Surface::Kick,
                            timestamp: 0,
                        },
                        Note {
                            value: NoteValue::Quarter,
                            surface: Surface::Hihat,
                            timestamp: 0,
                        },
                        Note {
                            value: NoteValue::Eighth,
                            surface: Surface::Hihat,
                            timestamp: 32,
                        },
                        Note {
                            value: NoteValue::Quarter,
                            surface: Surface::Snare,
                            timestamp: 64,
                        },
                        Note {
                            value: NoteValue::Quarter,
                            surface: Surface::Hihat,
                            timestamp: 64,
                        },
                        Note {
                            value: NoteValue::Eighth,
                            surface: Surface::Hihat,
                            timestamp: 96,
                        },
                        Note {
                            value: NoteValue::Quarter,
                            surface: Surface::Kick,
                            timestamp: 128,
                        },
                        Note {
                            value: NoteValue::Quarter,
                            surface: Surface::Hihat,
                            timestamp: 128,
                        },
                        Note {
                            value: NoteValue::Eighth,
                            surface: Surface::Hihat,
                            timestamp: 160,
                        },
                        Note {
                            value: NoteValue::Quarter,
                            surface: Surface::Snare,
                            timestamp: 192,
                        },
                        Note {
                            value: NoteValue::Quarter,
                            surface: Surface::Hihat,
                            timestamp: 192,
                        },
                        Note {
                            value: NoteValue::Eighth,
                            surface: Surface::Hihat,
                            timestamp: 224,
                        },
                    ],
                },
                Measure {
                    timestamp: 256,
                    time_signature: TimeSignature {
                        notes_per_measure: 4,
                        note_value: NoteValue::Quarter,
                    },
                    notes: vec![
                        Note {
                            value: NoteValue::Quarter,
                            surface: Surface::Snare,
                            timestamp: 256,
                        },
                        Note {
                            value: NoteValue::Quarter,
                            surface: Surface::Snare,
                            timestamp: 320,
                        },
                        Note {
                            value: NoteValue::Quarter,
                            surface: Surface::Snare,
                            timestamp: 384,
                        },
                        Note {
                            value: NoteValue::Quarter,
                            surface: Surface::Snare,
                            timestamp: 448,
                        },
                    ],
                },
            ],
        })
    })
}

pub fn use_score_manager() -> Signal<Score> {
    use_context::<Signal<Score>>()
}

use dioxus::prelude::*;
use staff::Staff;

use crate::{
    data::{
        measure::Measure,
        note::{Note, NoteValue, Surface},
        score::Score as ScoreData,
    },
    ui::hooks::use_size,
};

mod measure;
mod measure_scale;
mod note;
mod preview_note;
mod staff;

#[component]
pub fn Score() -> Element {
    let (width, _, handle_on_resize) = use_size();
    let mut score = use_score();
    let measures_per_staff = ((width / 350.0).floor() as u32).max(1);
    let playhead_timestamp = use_signal(|| 0u32);
    let playhead_staff_number = playhead_timestamp / (measures_per_staff * 256);
    let chunked_measures: Vec<Vec<Measure>> = score()
        .measures
        .chunks(measures_per_staff as usize)
        .map(|chunk| chunk.to_vec())
        .collect();

    let handle_remove_note = move |note: Note| {
        let updated_score = ScoreData {
            measures: score()
                .measures
                .into_iter()
                .map(|mut measure| {
                    if measure.timestamp <= note.timestamp
                        && note.timestamp - measure.timestamp < 256
                    {
                        measure.remove_note(note.clone());
                    }

                    measure
                })
                .collect(),
        };

        score.set(updated_score);
    };

    let handle_add_note = move |note: Note| {
        let updated_score = ScoreData {
            measures: score()
                .measures
                .into_iter()
                .map(|mut measure| {
                    if measure.timestamp <= note.timestamp
                        && note.timestamp - measure.timestamp < 256
                    {
                        measure.add_note(note.clone());
                    }

                    measure
                })
                .collect(),
        };

        score.set(updated_score);
    };

    rsx! {
        div { class: "score", onresize: handle_on_resize,

            {
                chunked_measures
                    .into_iter()
                    .enumerate()
                    .map(|(i, measures)| rsx! {
                        Staff {
                            width: width as f32,
                            height: 120.0,
                            measures,
                            handle_add_note,
                            handle_remove_note,
                            playhead_timestamp: if playhead_staff_number == i as u32 { Some(playhead_timestamp()) } else { None },
                        }
                    })
            }
        }
    }
}

fn use_score() -> Signal<ScoreData> {
    use_signal(|| ScoreData {
        measures: vec![
            Measure {
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
                timestamp: 0,
                length: 256,
            },
            Measure {
                timestamp: 256,
                length: 256,
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
}

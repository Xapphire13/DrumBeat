use dioxus::prelude::*;
use staff::Staff;

use crate::{
    data::{
        measure::Measure,
        note::{Note, NoteValue, Surface},
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
    let measures_per_staff = ((width / 350.0).floor() as usize).max(1);
    let chunked_measures: Vec<Vec<Measure>> = score()
        .chunks(measures_per_staff)
        .map(|chunk| chunk.to_vec())
        .collect();

    let handle_remove_note = move |note: Note| {
        let updated_score = score()
            .into_iter()
            .map(|mut measure| {
                if measure.timestamp <= note.timestamp && note.timestamp - measure.timestamp < 256 {
                    measure.remove_note(note.clone());
                }

                measure
            })
            .collect();

        score.set(updated_score);
    };

    let handle_add_note = move |note: Note| {
        let updated_score = score()
            .into_iter()
            .map(|mut measure| {
                if measure.timestamp <= note.timestamp && note.timestamp - measure.timestamp < 256 {
                    measure.add_note(note.clone());
                }

                measure
            })
            .collect();

        score.set(updated_score);
    };

    rsx! {
        div { class: "score", onresize: handle_on_resize,

            {chunked_measures.into_iter().map(|measures| rsx! {
                Staff {
                    width: width as f32,
                    height: 120.0,
                    measures,
                    handle_add_note,
                    handle_remove_note,
                }
            })}
        }
    }
}

fn use_score() -> Signal<Vec<Measure>> {
    use_signal(|| {
        vec![
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
            },
            Measure {
                timestamp: 256,
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
        ]
    })
}

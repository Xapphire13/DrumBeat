use dioxus::prelude::*;

use crate::{data::note::Note as NoteData, ui::score::note::Note};

use super::measure_scale::MeasureScale;

/// Component that renders the notes of a measure
#[component]
pub fn Measure(scale: MeasureScale, notes: Vec<NoteData>) -> Element {
    rsx! {
        g {
            {
                notes
                    .iter()
                    .filter_map(|note| {
                        let drum_piece = note.drum_piece()?;
                        Some(rsx! {
                            Note {
                                value: note.duration.value,
                                drum_piece,
                                position: scale.get_position(note.timestamp, drum_piece),
                                scale: scale.clone(),
                            }
                        })
                    })
            }
        }
    }
}

use dioxus::prelude::*;

use crate::{data::note::Note as NoteData, ui::score::note::Note};

use super::measure_scale::MeasureScale;

#[component]
pub fn Measure(scale: MeasureScale, notes: Vec<NoteData>) -> Element {
    rsx! {
        g {
            {
                notes
                    .iter()
                    .map(|note| {
                        let position = scale.get_position(note);
                        rsx! {
                            Note {
                                value: note.value.clone(),
                                surface: note.surface.clone(),
                                position,
                                scale: scale.clone(),
                            }
                        }
                    })
            }
        }
    }
}

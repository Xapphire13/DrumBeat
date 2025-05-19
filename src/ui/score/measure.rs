use dioxus::prelude::*;

use crate::{
    data::{note::Note as NoteData, position::Position},
    ui::{
        app_mode::AppMode,
        score::{note::Note, preview_note::PreviewNote},
    },
};

use super::measure_scale::MeasureScale;

#[component]
pub fn Measure(
    scale: MeasureScale,
    notes: Vec<NoteData>,
    mouse_position: Option<Position>,
) -> Element {
    let app_mode = use_context::<Signal<AppMode>>();
    let is_edit_mode;
    let preview_note;

    match (app_mode(), mouse_position) {
        (AppMode::Edit { note_value }, Some(mouse_position)) => {
            is_edit_mode = true;
            preview_note = Some(scale.get_nearest_note(mouse_position, note_value));
        }
        _ => {
            is_edit_mode = false;
            preview_note = None;
        }
    };

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

            if is_edit_mode {
                if let Some(Some(preview_note)) = preview_note {
                    PreviewNote {
                        position: scale.get_position(&preview_note),
                        scale,
                    }
                }
            }
        }
    }
}

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
    on_preview_note_created: EventHandler<Option<NoteData>>,
) -> Element {
    let app_mode = use_context::<Signal<AppMode>>();
    let preview_note = if let (AppMode::Edit { note_value }, Some(mouse_position)) =
        (app_mode(), mouse_position)
    {
        let nearest_note = scale.get_nearest_note(mouse_position, note_value);

        if nearest_note.as_ref().is_some_and(|nearest_note| {
            notes.iter().any(|note| {
                note.surface == nearest_note.surface && note.timestamp == nearest_note.timestamp
            })
        }) {
            None
        } else {
            nearest_note
        }
    } else {
        None
    };

    use_effect(use_reactive(&preview_note, move |preview_note| {
        on_preview_note_created(preview_note);
    }));

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

            if let (AppMode::Edit { .. }, Some(preview_note)) = (app_mode(), preview_note) {
                PreviewNote { position: scale.get_position(&preview_note), scale }
            }
        }
    }
}

use dioxus::prelude::*;

use crate::{
    data::{
        app_mode::{AppMode, EditMode},
        note::{Note as NoteData, NoteValue},
        position::Position,
    },
    ui::score::{
        note::{Note, NoteColor},
        preview_note::PreviewNote,
    },
};

use super::measure_scale::MeasureScale;

#[component]
pub fn Measure(
    scale: MeasureScale,
    notes: Vec<NoteData>,
    mouse_position: Option<Position>,
    on_preview_note_created: Callback<Option<NoteData>>,
) -> Element {
    let app_mode = use_context::<Signal<AppMode>>();
    let preview_note = if let (
        AppMode::Edit {
            note_value,
            edit_mode,
        },
        Some(mouse_position),
    ) = (app_mode(), mouse_position)
    {
        scale.get_nearest_note(
            mouse_position,
            match edit_mode {
                EditMode::Add => note_value,
                EditMode::Remove => NoteValue::ThirtySecond,
            },
        )
    } else {
        None
    };
    let preview_note_is_existing_note = preview_note.as_ref().is_some_and(|nearest_note| {
        notes.iter().any(|note| {
            note.surface == nearest_note.surface && note.timestamp == nearest_note.timestamp
        })
    });

    use_effect(use_reactive(&preview_note, move |preview_note| {
        if let AppMode::Edit { .. } = app_mode() {
            on_preview_note_created(preview_note);
        }
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
                                color: match (
                                    app_mode(),
                                    preview_note
                                        .as_ref()
                                        .is_some_and(|preview_note| {
                                            preview_note.surface == note.surface
                                                && preview_note.timestamp == note.timestamp
                                        }),
                                ) {
                                    (AppMode::Edit { edit_mode: EditMode::Remove, .. }, true) => Some(NoteColor::Red),
                                    _ => None,
                                },
                            }
                        }
                    })
            }

            if let (AppMode::Edit { edit_mode: EditMode::Add, .. }, Some(preview_note), false) = (
                app_mode(),
                preview_note,
                preview_note_is_existing_note
            )
            {
                PreviewNote { position: scale.get_position(&preview_note), scale }
            }
        }
    }
}

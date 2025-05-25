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

/// Component that renders a measure with notes and potential preview notes.
///
/// This component is responsible for:
/// - Rendering existing notes
/// - Showing a preview note at the mouse position when in Add mode
/// - Highlighting notes to be removed when in Remove mode
/// - Calling back when a preview note is created
#[component]
pub fn Measure(
    scale: MeasureScale,
    notes: Vec<NoteData>,
    mouse_position: Option<Position>,
    on_preview_note_created: Callback<Option<NoteData>>,
) -> Element {
    let app_mode = use_context::<Signal<AppMode>>();
    let preview_note = calculate_preview_note(&app_mode(), &scale, mouse_position);
    let preview_note_overlaps_with_existing_note = is_existing_note(&preview_note, &notes);
    // Sort note to be removed after the rest
    notes.sort_by(|left, right| {
        is_note_to_be_removed(&app_mode(), &preview_note, left).cmp(&is_note_to_be_removed(
            &app_mode(),
            &preview_note,
            right,
        ))
    });

    // Notify parent component when preview note changes
    use_effect(use_reactive(&preview_note, move |preview_note| {
        if matches!(app_mode(), AppMode::Edit { .. }) {
            on_preview_note_created(preview_note);
        }
    }));

    rsx! {
        g {
            // Render existing notes
            {
                notes
                    .iter()
                    .map(|note| {
                        rsx! {
                            Note {
                                value: note.value.clone(),
                                surface: note.surface.clone(),
                                position: scale.get_position(note),
                                scale: scale.clone(),
                                color: determine_note_color(&app_mode(), &preview_note, note),
                            }
                        }
                    })
            }

            // Render preview note if in Add mode and not overlapping an existing note
            {
                should_show_preview_note(
                        &app_mode(),
                        &preview_note,
                        preview_note_overlaps_with_existing_note,
                    )
                    .then(|| {
                        let preview = preview_note.as_ref().unwrap();
                        rsx! {
                            PreviewNote { position: scale.get_position(preview), scale: scale.clone() }
                        }
                    })
            }
        }
    }
}

/// Calculates the preview note based on app mode and mouse position
fn calculate_preview_note(
    app_mode: &AppMode,
    scale: &MeasureScale,
    mouse_position: Option<Position>,
) -> Option<NoteData> {
    match (app_mode, mouse_position) {
        (
            AppMode::Edit {
                note_value,
                edit_mode,
            },
            Some(mouse_pos),
        ) => {
            let note_value_to_use = match edit_mode {
                EditMode::Add => note_value.clone(),
                EditMode::Remove => NoteValue::ThirtySecond, // Use smallest note value for removal
            };

            scale.get_nearest_note(mouse_pos, note_value_to_use)
        }
        _ => None,
    }
}

/// Checks if the preview note overlaps with any existing note
fn is_existing_note(preview_note: &Option<NoteData>, notes: &[NoteData]) -> bool {
    preview_note.as_ref().is_some_and(|preview| {
        notes
            .iter()
            .any(|note| note.surface == preview.surface && note.timestamp == preview.timestamp)
    })
}

/// Determines if a note should be highlighted for removal
fn is_note_to_be_removed(
    app_mode: &AppMode,
    preview_note: &Option<NoteData>,
    note: &NoteData,
) -> bool {
    if let AppMode::Edit {
        edit_mode: EditMode::Remove,
        ..
    } = app_mode
    {
        preview_note.as_ref().is_some_and(|preview| {
            preview.surface == note.surface && preview.timestamp == note.timestamp
        })
    } else {
        false
    }
}

/// Determines the color for a note based on app mode and preview state
fn determine_note_color(
    app_mode: &AppMode,
    preview_note: &Option<NoteData>,
    note: &NoteData,
) -> Option<NoteColor> {
    if is_note_to_be_removed(app_mode, preview_note, note) {
        Some(NoteColor::Red)
    } else {
        None
    }
}

/// Determines if a preview note should be shown
fn should_show_preview_note(
    app_mode: &AppMode,
    preview_note: &Option<NoteData>,
    preview_note_is_existing_note: bool,
) -> bool {
    matches!(
        app_mode,
        AppMode::Edit {
            edit_mode: EditMode::Add,
            ..
        }
    ) && preview_note.is_some()
        && !preview_note_is_existing_note
}

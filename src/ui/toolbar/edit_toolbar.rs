use dioxus::prelude::*;

use crate::data::{
    app_mode::{AppMode, EditMode},
    note::NoteValue,
};

#[component]
pub fn EditToolbar() -> Element {
    let mut app_mode = use_context::<Signal<AppMode>>();

    let handle_on_note_value_pressed = |note_value: NoteValue| {
        move |_: Event<MouseData>| {
            app_mode.set(match app_mode() {
                AppMode::Edit { edit_mode, .. } => AppMode::Edit {
                    note_value: note_value.clone(),
                    edit_mode,
                },
                current_mode @ AppMode::View => current_mode,
            });
        }
    };

    let handle_set_edit_mode = |edit_mode: EditMode| {
        move |_: Event<MouseData>| {
            app_mode.set(match app_mode() {
                AppMode::Edit { note_value, .. } => AppMode::Edit {
                    note_value,
                    edit_mode: edit_mode.clone(),
                },
                current_mode @ AppMode::View => current_mode,
            });
        }
    };

    rsx! {
        div { class: "toolbar",

            div {
                button { onclick: handle_set_edit_mode(EditMode::Add), "add" }
                button { onclick: handle_set_edit_mode(EditMode::Remove), "remove" }
            }

            div {
                button { onclick: handle_on_note_value_pressed(NoteValue::Quarter), "1/4" }
                button { onclick: handle_on_note_value_pressed(NoteValue::Eighth), "1/8" }
                button { onclick: handle_on_note_value_pressed(NoteValue::Sixteenth), "1/16" }
                button { onclick: handle_on_note_value_pressed(NoteValue::ThirtySecond),
                    "1/32"
                }
            }
        }
    }
}

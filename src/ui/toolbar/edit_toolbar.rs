use dioxus::prelude::*;

use crate::data::{app_mode::AppMode, note::NoteValue};

#[component]
pub fn EditToolbar() -> Element {
    let mut app_mode = use_context::<Signal<AppMode>>();

    let handle_on_note_value_pressed = |note_value: NoteValue| {
        move |_: Event<MouseData>| {
            app_mode.set(AppMode::Edit {
                note_value: note_value.clone(),
            });
        }
    };

    rsx! {
        div { class: "toolbar",

            div {
                button { "add" }
                button { "remove" }
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

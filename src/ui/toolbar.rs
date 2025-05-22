mod edit_toolbar;
mod view_toolbar;

use crate::data::{app_mode::AppMode, note::NoteValue};
use dioxus::prelude::*;
use edit_toolbar::EditToolbar;
use view_toolbar::ViewToolbar;

#[component]
pub fn Toolbar() -> Element {
    let mut app_mode = use_context::<Signal<AppMode>>();

    rsx! {
        div { class: "toolbar",

            button {
                onclick: move |_| {
                    app_mode
                        .set(
                            match app_mode() {
                                AppMode::View => {
                                    AppMode::Edit {
                                        note_value: NoteValue::Eighth,
                                    }
                                }
                                AppMode::Edit { .. } => AppMode::View,
                            },
                        );
                },
                "mode {app_mode}"
            }

            {
                match app_mode() {
                    AppMode::View => rsx! {
                        ViewToolbar {}
                    },
                    AppMode::Edit { .. } => rsx! {
                        EditToolbar {}
                    },
                }
            }
        }
    }
}

mod edit_toolbar;
mod view_toolbar;

use crate::ui::app_mode::AppMode;
use dioxus::prelude::*;
use edit_toolbar::EditToolbar;
use view_toolbar::ViewToolbar;

#[component]
pub fn Toolbar() -> Element {
    let mut mode = use_signal(|| AppMode::View);

    rsx! {
        div { class: "toolbar",

            button {
                onclick: move |_| {
                    mode.set(
                        match mode() {
                            AppMode::View => AppMode::Edit,
                            AppMode::Edit => AppMode::View,
                        },
                    );
                },
                "mode {mode}"
            }

            {
                match mode() {
                    AppMode::View => rsx! {
                        ViewToolbar {}
                    },
                    AppMode::Edit => rsx! {
                        EditToolbar {}
                    },
                }
            }
        }
    }
}

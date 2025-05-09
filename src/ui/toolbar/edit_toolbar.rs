use dioxus::prelude::*;

#[component]
pub fn EditToolbar() -> Element {
    rsx! {
        div { class: "toolbar",

            div {
                button { "add" }
                button { "remove" }
            }

            div {
                button { "1/4" }
                button { "1/8" }
                button { "1/16" }
                button { "1/32" }
            }
        }
    }
}

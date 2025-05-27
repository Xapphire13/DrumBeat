use dioxus::prelude::*;

#[component]
pub fn PlaybackBar() -> Element {
    rsx! {
        div { class: "playback-bar-container",

            div { class: "playback-bar",
                "Measure:"
                input {type: "text", value: "1"}
                "/ ?"
                button { "<" }
                button { ">" }
            }
        }
    }
}

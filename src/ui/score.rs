use dioxus::prelude::*;
use staff::Staff;

mod measure;
mod measure_scale;
mod note;
mod preview_note;
mod staff;

#[component]
pub fn Score() -> Element {
    rsx! {
        div { class: "score",
            Staff {}
            Staff {}
        }
    }
}

use dioxus::prelude::*;
use staff::Staff;

mod measure;
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

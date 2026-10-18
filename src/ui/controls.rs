//! Small controls shared by the metronome's settings

use dioxus::prelude::*;

/// An on/off switch
#[component]
pub fn Switch(checked: bool, label: String, onchange: EventHandler<bool>) -> Element {
    rsx! {
        button {
            class: if checked { "switch on" } else { "switch" },
            role: "switch",
            aria_checked: checked,
            aria_label: label,
            onclick: move |_| onchange.call(!checked),
            span { class: "switch-thumb" }
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct SegmentOption {
    pub label: String,
    pub selected: bool,
}

impl SegmentOption {
    pub fn new(label: impl Into<String>, selected: bool) -> Self {
        Self {
            label: label.into(),
            selected,
        }
    }
}

/// A row of buttons picking one option, reporting the index of the one
/// picked
#[component]
pub fn Segmented(
    label: String,
    options: Vec<SegmentOption>,
    onselect: EventHandler<usize>,
) -> Element {
    rsx! {
        div { class: "segmented", role: "radiogroup", aria_label: label,
            for (i , option) in options.into_iter().enumerate() {
                button {
                    key: "{i}",
                    class: if option.selected { "segment selected" } else { "segment" },
                    role: "radio",
                    aria_checked: option.selected,
                    onclick: move |_| onselect.call(i),
                    "{option.label}"
                }
            }
        }
    }
}

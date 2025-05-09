use dioxus::prelude::*;

use super::{measure_scale::MeasureScale, note::Note};

#[component]
pub fn Measure(scale: MeasureScale, notes: Vec<Note>) -> Element {
    rsx! {
        g {
            {
                notes
                    .iter()
                    .map(|note| {
                        let (x, y) = scale.get_position(note);
                        rsx! {
                            circle { r: 7, cx: x, cy: y }
                        }
                    })
            }
        }
    }
}

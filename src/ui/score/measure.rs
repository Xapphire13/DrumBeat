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
                            circle {
                                // Minus 2 to account for the line stroke
                                r: (scale.line_height - 2.) / 2.,
                                cx: x,
                                cy: y,
                            }
                        }
                    })
            }
        }
    }
}

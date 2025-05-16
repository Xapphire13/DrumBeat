use dioxus::prelude::*;

use crate::data::{
    note::{NoteValue, Surface},
    position::Position,
};

use super::measure_scale::MeasureScale;

#[component]
pub fn Note(
    value: NoteValue,
    surface: Surface,
    position: Position,
    scale: MeasureScale,
) -> Element {
    let Position { x, y } = position;

    rsx! {
        circle {
            // Minus 2 to account for the line stroke
            r: (scale.line_height - 2.) / 2.,
            cx: x,
            cy: y,
        }
    }
}

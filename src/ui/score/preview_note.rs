use dioxus::prelude::*;

use crate::data::position::Position;

use super::measure_scale::MeasureScale;

/**
* A preview note that appears when hovering over the score to indicate where a note would be placed
*/
#[component]
pub fn PreviewNote(position: Position, scale: MeasureScale) -> Element {
    let Position { x, y } = position;
    let size = (scale.line_height - 2.) / 2.;

    let fill_color = "#AAAAAA";
    let stroke_color = "#DDDDDD";

    // Diamond shape points
    let points = format!(
        "{},{} {},{} {},{} {},{}",
        // top point
        x,
        y - size,
        // right point
        x + size,
        y,
        // bottom point
        x,
        y + size,
        // left point
        x - size,
        y
    );

    rsx! {
        g {
            polygon {
                points,
                fill: fill_color,
                stroke: stroke_color,
                stroke_width: 1.5,
            }
        }
    }
}

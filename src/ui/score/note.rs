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
    // Minus 2 to account for the line stroke
    let size = (scale.line_height - 2.) / 2.;

    match surface {
        Surface::Hihat => {
            rsx! {
                g {
                    line {
                        x1: x - size,
                        y1: y - size,
                        x2: x + size,
                        y2: y + size,
                        stroke: "black",
                        stroke_width: 1.5,
                    }
                    line {
                        x1: x - size,
                        y1: y + size,
                        x2: x + size,
                        y2: y - size,
                        stroke: "black",
                        stroke_width: 1.5,
                    }
                }
            }
        }
        _ => {
            rsx! {
                circle {
                    r: size,
                    cx: x,
                    cy: y,
                }
            }
        }
    }
}

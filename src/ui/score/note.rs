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
    // Size calculations for note shapes
    let size = (scale.line_height - 2.) / 2.;

    // Calculate beam line position
    let beam_line_number = scale.get_beam_line_number(value);
    let beam_y = scale.get_y(beam_line_number);

    rsx! {
        g {
            match surface {
                Surface::Hihat => {
                    rsx! {
                        // Note body - X shape
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
                        // Stem line to beam for hihat (from top right corner)
                        line {
                            x1: x + size,
                            y1: y - size,
                            x2: x + size,
                            y2: beam_y,
                            stroke: "black",
                            stroke_width: 1.5,
                        }
                    }
                }
                _ => {
                    let ellipse_rx = scale.line_height / 2.0;
                    let ellipse_ry = scale.line_height * 0.3;
                    rsx! {
                        // Note body - ellipse (tilted music note)
                        ellipse {
                            rx: ellipse_rx,
                            ry: ellipse_ry,
                            cx: x,
                            cy: y,
                            transform: format!("rotate(-30 {} {})", x, y),
                            fill: "black",
                        }
                        // Stem line to beam for other notes
                        line {
                            x1: x + size,
                            y1: y - 1.,
                            x2: x + size,
                            y2: beam_y,
                            stroke: "black",
                            stroke_width: 1.5,
                        }
                    }
                }
            }
        }
    }
}

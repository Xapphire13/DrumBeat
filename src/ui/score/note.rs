use dioxus::prelude::*;

use crate::data::{
    note::{NoteValue, Surface},
    position::Position,
};

use super::measure_scale::MeasureScale;

#[derive(Clone, PartialEq)]
pub enum NoteColor {
    Black,
    Red,
}

impl NoteColor {
    fn get_css_color(&self) -> &str {
        match self {
            NoteColor::Black => "black",
            NoteColor::Red => "red",
        }
    }
}

#[component]
pub fn Note(
    value: NoteValue,
    surface: Surface,
    position: Position,
    scale: MeasureScale,
    color: Option<NoteColor>,
) -> Element {
    let Position { x, y } = position;
    // Size calculations for note shapes
    let size = (scale.line_height - 2.) / 2.;

    // Calculate beam line position
    let beam_line_number = scale.get_beam_line_number(value);
    let beam_y = scale.get_y(beam_line_number);
    let fill_color = color.as_ref().unwrap_or(&NoteColor::Black).get_css_color();

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
                            stroke: fill_color,
                            stroke_width: 1.5,
                        }
                        line {
                            x1: x - size,
                            y1: y + size,
                            x2: x + size,
                            y2: y - size,
                            stroke: fill_color,
                            stroke_width: 1.5,
                        }
                        // Stem line to beam for hihat (from top right corner)
                        line {
                            x1: x + size,
                            y1: y - size,
                            x2: x + size,
                            y2: beam_y,
                            stroke: fill_color,
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
                            fill: fill_color,
                            stroke: fill_color,
                        }
                        // Stem line to beam for other notes
                        line {
                            x1: x + size,
                            y1: y - 1.,
                            x2: x + size,
                            y2: beam_y,
                            stroke: fill_color,
                            stroke_width: 1.5,
                        }
                    }
                }
            }
        }
    }
}

use dioxus::prelude::*;

use crate::data::note::{NoteValue, Rest as RestData};

use super::measure_scale::MeasureScale;

/// A rest, centred on `center_line`. A rest lasting the whole measure is
/// drawn in the middle of it
#[component]
pub fn Rest(
    scale: MeasureScale,
    rest: RestData,
    measure_start: u32,
    measure_length: u32,
    center_line: f32,
) -> Element {
    let line_height = scale.line_height;
    let unit = scale.unit;
    let is_whole_measure =
        rest.timestamp == measure_start && rest.duration.get_ticks() >= measure_length;
    let x = if is_whole_measure {
        let (left, right) = scale.outer_x_bounds;
        (left + right) / 2.
    } else {
        scale.get_x(rest.timestamp)
    };
    let y = scale.get_y(center_line);
    // Path coordinates below are in line heights from (x, y), y down
    let point = |dx: f32, dy: f32| format!("{} {}", x + dx * line_height, y + dy * line_height);

    let glyph = match rest.duration.value {
        // Hangs from the line above the centre
        NoteValue::Whole => rsx! {
            rect {
                x: x - 0.5 * unit,
                y: y - 1.0 * line_height,
                width: unit,
                height: 0.5 * line_height,
                fill: "black",
                stroke: "none",
            }
        },
        // Sits on the line below the centre
        NoteValue::Half => rsx! {
            rect {
                x: x - 0.5 * unit,
                y: y + 0.5 * line_height,
                width: unit,
                height: 0.5 * line_height,
                fill: "black",
                stroke: "none",
            }
        },
        NoteValue::Quarter => {
            let d = format!(
                "M {} L {} L {} L {} Q {} {}",
                point(-0.15, -1.4),
                point(0.25, -0.8),
                point(-0.15, -0.25),
                point(0.25, 0.3),
                point(-0.35, 0.15),
                point(0.0, 1.0),
            );
            rsx! {
                path {
                    d,
                    fill: "none",
                    stroke: "black",
                    stroke_width: 0.2 * line_height,
                    stroke_linejoin: "round",
                    stroke_linecap: "round",
                }
            }
        }
        value => {
            let flags = match value {
                NoteValue::Eighth => 1,
                NoteValue::Sixteenth => 2,
                NoteValue::ThirtySecond => 3,
                _ => 4,
            };
            // A slanted stem with a dot and curl for each flag
            let height = 0.9 + 0.5 * (flags - 1) as f32;
            let top = -height / 2. - 0.2;
            let stem_x = |dy: f32| 0.3 - 0.3 * (dy - top);
            let flag_paths: Vec<(String, (f32, f32))> = (0..flags)
                .map(|flag| {
                    let flag_y = top + 0.5 * flag as f32;
                    let flag_x = stem_x(flag_y);
                    let dot = (flag_x - 0.55, flag_y + 0.1);
                    let d = format!(
                        "M {} Q {} {}",
                        point(dot.0, dot.1 + 0.1),
                        point(flag_x - 0.2, flag_y + 0.3),
                        point(flag_x, flag_y),
                    );
                    (d, (x + dot.0 * line_height, y + dot.1 * line_height))
                })
                .collect();
            let stem = format!(
                "M {} L {}",
                point(stem_x(top), top),
                point(stem_x(top + height), top + height),
            );
            rsx! {
                path {
                    d: stem,
                    fill: "none",
                    stroke: "black",
                    stroke_width: 1.5,
                }
                for (d, (cx, cy)) in flag_paths {
                    path {
                        d,
                        fill: "none",
                        stroke: "black",
                        stroke_width: 1.5,
                    }
                    circle {
                        cx,
                        cy,
                        r: 0.17 * line_height,
                        fill: "black",
                        stroke: "none",
                    }
                }
            }
        }
    };

    rsx! {
        g {
            {glyph}
            for dot in 0..rest.duration.dots {
                circle {
                    cx: x + (0.7 + 0.45 * f32::from(dot)) * unit,
                    cy: y - 0.25 * line_height,
                    r: 0.15 * unit,
                    fill: "black",
                    stroke: "none",
                }
            }
        }
    }
}

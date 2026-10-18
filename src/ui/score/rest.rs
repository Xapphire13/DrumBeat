use dioxus::prelude::*;

use crate::data::note::{NoteValue, Rest as RestData};

use super::measure_scale::MeasureScale;

const QUARTER_REST_PATH: &str = "M 0.1754 -0.2385 C 0.0839 -0.1294 0.0381 -0.0478 0.0381 0.0061 \
     C 0.0381 0.0582 0.0817 0.1362 0.1688 0.2404 L 0.1415 0.2789 \
     C 0.0977 0.2532 0.0600 0.2404 0.0287 0.2404 C -0.0121 0.2404 -0.0324 0.2648 -0.0324 0.3137 \
     C -0.0324 0.3639 -0.0102 0.4138 0.0343 0.4633 L 0.0099 0.5000 \
     C -0.1136 0.4084 -0.1754 0.3269 -0.1754 0.2554 C -0.1754 0.2190 -0.1629 0.1902 -0.1378 0.1689 \
     C -0.1146 0.1488 -0.0845 0.1388 -0.0475 0.1388 C -0.0237 0.1388 0.0027 0.1450 0.0315 0.1576 \
     L -0.1575 -0.0936 C -0.0679 -0.1726 -0.0230 -0.2441 -0.0230 -0.3081 \
     C -0.0230 -0.3589 -0.0535 -0.4229 -0.1143 -0.5000 L -0.0390 -0.5000 Z";

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
        // Crotchet rest outline from Wikimedia Commons
        // (Crotchet_rest_alt_plain-svg.svg), normalised to unit height around
        // its centre
        NoteValue::Quarter => {
            let height = 2.8 * line_height;
            let transform = format!("translate({x} {}) scale({height})", y - 0.2 * line_height);
            rsx! {
                path {
                    d: QUARTER_REST_PATH,
                    transform,
                    fill: "black",
                    stroke: "none",
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
                    stroke_width: 0.12 * line_height,
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

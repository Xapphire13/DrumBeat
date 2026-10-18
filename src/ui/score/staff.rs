use dioxus::prelude::*;

use crate::{
    data::{
        measure::{Fill, Measure as MeasureData},
        score::{Hairpin, HairpinKind},
    },
    ui::{
        hooks::{MousePositionData, use_mouse_position},
        providers::{playback_manager::use_playback_manager, score_manager::use_score_manager},
        score::{
            StaffDimensions, StaffLayout,
            measure::Measure,
            measure_scale::MeasureScale,
            staff::{
                phantom_playhead::{PhantomPlayhead, use_phantom_playhead_data},
                playhead::Playhead,
                staff_lines::StaffLines,
            },
        },
    },
};

mod phantom_playhead;
mod playhead;
mod staff_lines;

/// The highest and lowest line numbers drawn, leaving room above the staff
/// for upward stems, beams and measure numbers, and below it for downward
/// stems and beams. The staff itself spans lines -0.5 to 3.5
const HIGHEST_LINE: f32 = 7.0;
const LOWEST_LINE: f32 = -3.5;
/// Top and bottom staff lines
const TOP_STAFF_LINE: f32 = 3.5;
const BOTTOM_STAFF_LINE: f32 = -0.5;

/// Extra room below a staff with hairpins, in lines
const HAIRPIN_BAND: f32 = 1.5;
/// How wide a hairpin opens, in lines
const HAIRPIN_OPENING: f32 = 0.9;

/// The staff's height in lines
fn height_in_lines(has_hairpins: bool) -> f32 {
    HIGHEST_LINE - LOWEST_LINE + if has_hairpins { HAIRPIN_BAND } else { 0.0 }
}

/// Height of a staff drawn with the given line height
pub fn height(line_height: f32, has_hairpins: bool) -> f32 {
    height_in_lines(has_hairpins) * line_height
}

#[component]
pub fn Staff(dimensions: StaffDimensions, layout: StaffLayout) -> Element {
    let StaffDimensions { width, height } = dimensions.clone();
    let StaffLayout {
        first_measure_number,
        measures,
        widths,
        start,
        left,
        hairpins,
    } = layout;
    // A short last staff doesn't fill the whole width
    let content_width: f32 = widths.iter().sum();
    let mut playback_manager = use_playback_manager();
    let score = use_score_manager();
    let MousePositionData {
        mouse_position,
        handle_on_mouse_move,
        handle_on_mouse_leave,
    } = use_mouse_position();
    let line_height = height / height_in_lines(!hairpins.is_empty());
    let baseline = HIGHEST_LINE * line_height;
    let scales = use_scales(&measures, &widths, left, baseline, line_height);
    let phantom_playhead_data = use_phantom_playhead_data(&mouse_position, &scales(), &measures);

    let handle_on_click = {
        let phantom_playhead_data = phantom_playhead_data.clone();
        move |_: MouseEvent| {
            if let Some(data) = phantom_playhead_data.clone() {
                playback_manager.update_playhead(data.closest_timestamp, &score());
            }
        }
    };

    rsx! {
        div {
            class: "staff-container",
            onmousemove: handle_on_mouse_move,
            onmouseleave: handle_on_mouse_leave,
            onclick: handle_on_click,
            style: "height: {height}px",

            svg { class: "staff", width: "{width}px", height: "{height}px",
                for (measure , scale) in measures.iter().zip(scales()) {
                    if measure.fill != Fill::Full {
                        InvalidMeasure { scale, fill: measure.fill, repeat_end: measure.repeat_end }
                    }
                }

                StaffLines { line_height, baseline, x_bounds: (start, left + content_width) }

                // Open the staff with a bar line and a percussion clef
                line {
                    x1: "{start}",
                    y1: "{baseline - BOTTOM_STAFF_LINE * line_height}",
                    x2: "{start}",
                    y2: "{baseline - TOP_STAFF_LINE * line_height}",
                }
                for offset in [0.6, 1.15] {
                    rect {
                        x: start + offset * line_height,
                        y: baseline - 2.5 * line_height,
                        width: 0.3 * line_height,
                        height: 2.0 * line_height,
                        fill: "black",
                        stroke: "none",
                    }
                }

                for hairpin in hairpins {
                    HairpinMark {
                        hairpin,
                        measures: measures.clone(),
                        scales: scales(),
                    }
                }

                Playhead {
                    measures: measures.clone(),
                    scales,
                    staff_dimensions: dimensions.clone(),
                }

                if let Some(data) = phantom_playhead_data {
                    PhantomPlayhead { data, staff_dimensions: dimensions }
                }

                // Measures
                {
                    let number_of_measures = measures.len();
                    let performance = score.read().performance.clone();
                    measures
                        .into_iter()
                        .enumerate()
                        .filter_map(move |(i, measure)| {
                            let scale = scales.get(i)?;
                            let (left, right) = scale.outer_x_bounds;
                            let top = scale.get_y(TOP_STAFF_LINE);
                            let bottom = scale.get_y(BOTTOM_STAFF_LINE);
                            let measure_number = format_measure_numbers(
                                &performance.measure_numbers(first_measure_number - 1 + i),
                            );
                            Some(rsx! {
                                // Draw vertical line between measures
                                if i > 0 {
                                    line {
                                        x1: "{left}",
                                        y1: "{bottom}",
                                        x2: "{left}",
                                        y2: "{top}",
                                    }
                                }
                                // Close the staff after its last measure
                                if i + 1 == number_of_measures {
                                    line {
                                        x1: "{right - 0.5}",
                                        y1: "{bottom}",
                                        x2: "{right - 0.5}",
                                        y2: "{top}",
                                    }
                                }
                                text {
                                    class: "measure-number-label",
                                    x: left + 2.,
                                    y: top - 0.35 * scale.unit,
                                    font_size: 0.75 * scale.unit,
                                    "{measure_number}"
                                }
                                RepeatSigns {
                                    scale: scale.clone(),
                                    repeat_start: measure.repeat_start,
                                    repeat_end: measure.repeat_end,
                                }
                                Measure {
                                    scale: scale.clone(),
                                    measure,
                                    stem_bounds: (LOWEST_LINE + 0.3, HIGHEST_LINE - 0.3),
                                }
                            })
                        })
                }
            }
        }
    }
}

/// A highlight behind a measure whose beats don't add up to its time
/// signature, with a note above the staff saying what's wrong
#[component]
fn InvalidMeasure(scale: MeasureScale, fill: Fill, repeat_end: Option<u32>) -> Element {
    let unit = scale.unit;
    let (left, right) = scale.outer_x_bounds;
    let top = scale.get_y(TOP_STAFF_LINE + 1.0);
    let bottom = scale.get_y(BOTTOM_STAFF_LINE - 1.0);
    let (label, description) = match fill {
        Fill::Short => (
            "Short bar",
            "The beats in this bar don't fill its time signature, so the rest of it is silent",
        ),
        Fill::Over { .. } | Fill::Full => (
            "Long bar",
            "The beats in this bar run past its time signature, so it has been lengthened to fit them",
        ),
    };
    // Keep clear of a repeat's play count, which sits in the same corner
    let label_right = right
        - 0.9 * unit
        - if repeat_end.is_some_and(|count| count != 2) {
            2.0 * unit
        } else {
            0.0
        };

    rsx! {
        rect {
            class: "invalid-measure",
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
            title { "{description}" }
        }
        text {
            class: "invalid-measure-label",
            x: label_right,
            y: scale.get_y(TOP_STAFF_LINE) - 0.35 * unit,
            font_size: 0.75 * unit,
            text_anchor: "end",
            title { "{description}" }
            "⚠ {label}"
        }
    }
}

/// Show the numbers a measure is played as, e.g. "5" or, when it repeats,
/// "1–4" or "97, 99, 101"
fn format_measure_numbers(numbers: &[usize]) -> String {
    let mut runs: Vec<(usize, usize)> = vec![];
    for &number in numbers {
        match runs.last_mut() {
            Some((_, last)) if *last + 1 == number => *last = number,
            _ => runs.push((number, number)),
        }
    }
    runs.iter()
        .map(|&(first, last)| {
            if first == last {
                format!("{first}")
            } else {
                format!("{first}–{last}")
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn use_scales(
    measures: &Vec<MeasureData>,
    widths: &Vec<f32>,
    left: f32,
    baseline: f32,
    line_height: f32,
) -> Memo<Vec<MeasureScale>> {
    use_memo(use_reactive(
        (measures, widths, &left, &baseline),
        move |(measures, widths, left, baseline)| {
            let mut left_bound = left;
            measures
                .iter()
                .zip(&widths)
                .map(|(measure, width)| {
                    let right_bound = left_bound + width;
                    let scale = MeasureScale::new(
                        (left_bound, right_bound),
                        baseline,
                        line_height,
                        measure,
                    );

                    left_bound = right_bound;
                    scale
                })
                .collect::<Vec<MeasureScale>>()
        },
    ))
}

/// A crescendo or decrescendo wedge below the staff, for the part of it on
/// this staff
#[component]
fn HairpinMark(hairpin: Hairpin, measures: Vec<MeasureData>, scales: Vec<MeasureScale>) -> Element {
    let (Some(first), Some(last), Some(scale)) =
        (measures.first(), measures.last(), scales.first())
    else {
        return rsx! {};
    };
    let x_at = |timestamp: u32| {
        measures
            .iter()
            .zip(&scales)
            .find(|(measure, _)| timestamp <= measure.end_timestamp())
            .map_or(0.0, |(_, scale)| scale.get_x(timestamp))
    };
    let start = hairpin.start.max(first.timestamp);
    let end = hairpin.end.min(last.end_timestamp());
    // How far open the wedge is at a timestamp
    let half_opening = |timestamp: u32| {
        let progress = (timestamp - hairpin.start) as f32 / (hairpin.end - hairpin.start) as f32;
        let open = match hairpin.kind {
            HairpinKind::Crescendo => progress,
            HairpinKind::Decrescendo => 1.0 - progress,
        };
        open * HAIRPIN_OPENING * scale.line_height / 2.
    };
    let (x1, x2) = (x_at(start), x_at(end));
    let (open1, open2) = (half_opening(start), half_opening(end));
    let y = scale.get_y(LOWEST_LINE - HAIRPIN_BAND / 2.);

    rsx! {
        polyline {
            points: "{x2},{y - open2} {x1},{y - open1}",
            fill: "none",
            stroke: "black",
            stroke_width: 1.2,
        }
        polyline {
            points: "{x2},{y + open2} {x1},{y + open1}",
            fill: "none",
            stroke: "black",
            stroke_width: 1.2,
        }
    }
}

/// Thick and thin bar lines with dots for a repeated passage's start and end,
/// and the number of times it is played when that isn't the usual twice
#[component]
fn RepeatSigns(scale: MeasureScale, repeat_start: bool, repeat_end: Option<u32>) -> Element {
    let unit = scale.unit;
    let (left, right) = scale.outer_x_bounds;
    let top = scale.get_y(TOP_STAFF_LINE);
    let bottom = scale.get_y(BOTTOM_STAFF_LINE);
    let dot_radius = 0.2 * unit;
    // The dots sit in the two middle spaces of the staff
    let dot_ys = [scale.get_y(1.0), scale.get_y(2.0)];

    rsx! {
        if repeat_start {
            line {
                x1: left + 2.,
                y1: bottom,
                x2: left + 2.,
                y2: top,
                stroke_width: 3,
            }
            for y in dot_ys {
                circle {
                    cx: left + 0.6 * unit,
                    cy: y,
                    r: dot_radius,
                    fill: "black",
                    stroke: "none",
                }
            }
        }
        if let Some(count) = repeat_end {
            line {
                x1: right - 2.,
                y1: bottom,
                x2: right - 2.,
                y2: top,
                stroke_width: 3,
            }
            for y in dot_ys {
                circle {
                    cx: right - 0.6 * unit,
                    cy: y,
                    r: dot_radius,
                    fill: "black",
                    stroke: "none",
                }
            }
            if count != 2 {
                text {
                    class: "repeat-count",
                    x: right - 0.9 * unit,
                    y: top - 0.35 * unit,
                    font_size: 0.75 * unit,
                    text_anchor: "end",
                    "×{count}"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measure_numbers_collapse_into_ranges() {
        assert_eq!(format_measure_numbers(&[5]), "5");
        assert_eq!(format_measure_numbers(&[1, 2, 3, 4]), "1–4");
        assert_eq!(format_measure_numbers(&[97, 99, 101]), "97, 99, 101");
    }
}

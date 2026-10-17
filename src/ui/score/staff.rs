use dioxus::prelude::*;

use crate::{
    data::measure::Measure as MeasureData,
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

const NUMBER_OF_LINES: u8 = 5;

pub fn line_height(staff_height: f32) -> f32 {
    staff_height / 8.
}

#[component]
pub fn Staff(dimensions: StaffDimensions, layout: StaffLayout) -> Element {
    let StaffDimensions { width, height } = dimensions.clone();
    let StaffLayout { measures, widths } = layout;
    // A short last staff doesn't fill the whole width
    let content_width: f32 = widths.iter().sum();
    let mut playback_manager = use_playback_manager();
    let score = use_score_manager();
    let MousePositionData {
        mouse_position,
        handle_on_mouse_move,
        handle_on_mouse_leave,
    } = use_mouse_position();
    let line_height = line_height(height);
    let scales = use_scales(&measures, &widths, height, line_height);
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
                StaffLines {
                    line_height,
                    number_of_lines: NUMBER_OF_LINES,
                    staff_dimensions: StaffDimensions {
                        width: content_width,
                        height,
                    },
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
                    measures
                        .into_iter()
                        .enumerate()
                        .filter_map(move |(i, measure)| {
                            let scale = scales.get(i)?;
                            let outer_left_bound = scale.outer_x_bounds.0;
                            Some(rsx! {
                                // Draw vertical line between measures
                                if i > 0 {
                                    line {
                                        x1: "{outer_left_bound}",
                                        y1: "{scale.get_y(-0.5)}",
                                        x2: "{outer_left_bound}",
                                        y2: "{scale.get_y(3.5)}",
                                    }
                                }
                                // Close the staff after its last measure
                                if i + 1 == number_of_measures {
                                    line {
                                        x1: "{scale.outer_x_bounds.1 - 0.5}",
                                        y1: "{scale.get_y(-0.5)}",
                                        x2: "{scale.outer_x_bounds.1 - 0.5}",
                                        y2: "{scale.get_y(3.5)}",
                                    }
                                }
                                Measure { scale: scale.clone(), notes: measure.notes }
                            })
                        })
                }
            }
        }
    }
}

fn use_scales(
    measures: &Vec<MeasureData>,
    widths: &Vec<f32>,
    height: f32,
    line_height: f32,
) -> Memo<Vec<MeasureScale>> {
    use_memo(use_reactive(
        (measures, widths, &height),
        move |(measures, widths, height)| {
            let mut left_bound = 0.0;
            measures
                .iter()
                .zip(&widths)
                .map(|(measure, width)| {
                    let right_bound = left_bound + width;
                    let scale = MeasureScale::new(
                        (left_bound, right_bound),
                        height - 1.5 * line_height,
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

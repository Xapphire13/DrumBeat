use dioxus::prelude::*;

use crate::{
    data::{
        app_mode::{AppMode, EditMode},
        measure::Measure as MeasureData,
        note::Note,
    },
    ui::{
        hooks::{MousePositionData, use_mouse_position},
        providers::{playback_manager::use_playback_manager, score_manager::use_score_manager},
        score::{
            StaffDimensions,
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

#[component]
pub fn Staff(
    dimensions: StaffDimensions,
    measures: Vec<MeasureData>,
    handle_remove_note: Callback<Note>,
    handle_add_note: Callback<Note>,
) -> Element {
    let StaffDimensions { width, height } = dimensions.clone();
    let mut playback_manager = use_playback_manager();
    let score = use_score_manager();
    let app_mode = use_context::<Signal<AppMode>>();
    let MousePositionData {
        mut mouse_position,
        handle_on_mouse_move,
        handle_on_mouse_leave,
    } = use_mouse_position();
    let mut preview_note: Signal<Option<Note>> = use_signal(|| None);
    let line_height = height / 8.;
    let scales = use_scales(&measures, width, height, line_height);
    let phantom_playhead_data =
        use_phantom_playhead_data(&app_mode, &mouse_position, &scales(), &measures);

    let handle_on_click = {
        let phantom_playhead_data = phantom_playhead_data.clone();
        move |_: MouseEvent| match app_mode() {
            AppMode::View => {
                if let Some(data) = phantom_playhead_data.clone() {
                    playback_manager.update_playhead(data.closest_timestamp, &score());
                }
            }
            AppMode::Edit { edit_mode, .. } => {
                if let Some(note) = preview_note() {
                    match edit_mode {
                        EditMode::Add => handle_add_note(note),
                        EditMode::Remove => handle_remove_note(note),
                    }

                    preview_note.set(None);
                }
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
                    staff_dimensions: StaffDimensions { width, height },
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
                    measures
                        .into_iter()
                        .enumerate()
                        .filter_map(|(i, measure)| {
                            let scale = scales.get(i)?;
                            let mouse_position = mouse_position
                                .take_if(|position| scale.is_within_bounds(position));
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
                                Measure {
                                    scale: scale.clone(),
                                    notes: measure.notes,
                                    mouse_position,
                                    on_preview_note_created: move |note| {
                                        preview_note.set(note);
                                    },
                                }
                            })
                        })
                }
            }
        }
    }
}

fn use_scales(
    measures: &Vec<MeasureData>,
    width: f32,
    height: f32,
    line_height: f32,
) -> Memo<Vec<MeasureScale>> {
    let num_measures = measures.len() as f32;

    use_memo(use_reactive(measures, move |measures| {
        let mut cumulative_timestamp = 0;
        measures
            .iter()
            .enumerate()
            .map(|(i, measure)| {
                let left_bound = (i as f32) * width / num_measures;
                let right_bound = (i as f32 + 1.) * width / num_measures;
                let padded_left_bound = left_bound + 1.5 * line_height;
                let padded_right_bound = right_bound - 0.5 * line_height;

                let scale = MeasureScale::new(
                    (left_bound, right_bound),
                    (padded_left_bound, padded_right_bound),
                    height - 1.5 * line_height,
                    line_height,
                    cumulative_timestamp,
                    measure.time_signature.clone(),
                );

                cumulative_timestamp += measure.get_length();
                scale
            })
            .collect::<Vec<MeasureScale>>()
    }))
}

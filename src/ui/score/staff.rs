use dioxus::{html::geometry::euclid::Point2D, prelude::*};

use crate::{
    data::{
        app_mode::{AppMode, EditMode},
        measure::Measure as MeasureData,
        note::Note,
        position::Position,
    },
    ui::{
        hooks::{PlaybackState, use_playback_state},
        score::{measure::Measure, measure_scale::MeasureScale},
    },
};

#[component]
pub fn Staff(
    width: f32,
    height: f32,
    measures: Vec<MeasureData>,
    handle_remove_note: Callback<Note>,
    handle_add_note: Callback<Note>,
    /// Playhead timestamp, if within this staff
    playhead_timestamp: Option<u32>,
) -> Element {
    let mut playback_state = use_playback_state();
    let app_mode = use_context::<Signal<AppMode>>();
    let mut mouse_position = use_signal::<Option<Position>>(|| None);
    let mut preview_note: Signal<Option<Note>> = use_signal(|| None);
    let line_height = height / 8.;
    let num_measures = measures.len() as f32;
    let scales = use_memo(use_reactive(&measures, move |measures| {
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
    }));

    // Calculate phantom playhead position for View mode
    let phantom_playhead = use_memo(use_reactive(
        (&app_mode, &mouse_position, &scales, &measures),
        move |(app_mode, mouse_position, scales, measures)| {
            if matches!(app_mode(), AppMode::View) {
                if let Some(Position { x, .. }) = mouse_position() {
                    let scales = scales();

                    // Collect all note timestamps from this staff
                    let mut note_timestamps = vec![];
                    for measure in measures.iter() {
                        for note in &measure.notes {
                            note_timestamps.push(note.timestamp);
                        }
                    }

                    if note_timestamps.is_empty() {
                        return None;
                    }

                    // Convert mouse x position to estimated timestamp
                    let mut mouse_timestamp = 0u32;
                    for (i, scale) in scales.iter().enumerate() {
                        if x >= scale.outer_x_bounds.0 && x <= scale.outer_x_bounds.1 {
                            if let Some(measure) = measures.get(i) {
                                let relative_x = (x - scale.outer_x_bounds.0)
                                    / (scale.outer_x_bounds.1 - scale.outer_x_bounds.0);
                                mouse_timestamp = measure.timestamp
                                    + (relative_x * measure.get_length() as f32) as u32;
                                break;
                            }
                        }
                    }

                    // Find closest actual note timestamp
                    if let Some(&closest_timestamp) = note_timestamps
                        .iter()
                        .min_by_key(|&&ts| (ts as i64 - mouse_timestamp as i64).abs())
                    {
                        // Find which scale contains this timestamp and get x position
                        for (i, scale) in scales.iter().enumerate() {
                            if let Some(measure) = measures.get(i) {
                                if closest_timestamp >= measure.timestamp
                                    && closest_timestamp < measure.timestamp + measure.get_length()
                                {
                                    return Some((
                                        closest_timestamp,
                                        scale.get_x(closest_timestamp),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            None
        },
    ));

    let handle_on_mouse_move = move |ev: MouseEvent| {
        let Point2D { x, y, .. } = ev.coordinates().element();
        mouse_position.set(Some(Position {
            x: x as f32,
            y: y as f32,
        }));
    };

    let handle_on_mouse_leave = move |_: MouseEvent| {
        mouse_position.set(None);
        preview_note.set(None);
    };

    let handle_on_click = move |_: MouseEvent| {
        if let Some(note) = preview_note() {
            match app_mode() {
                AppMode::Edit {
                    edit_mode: EditMode::Add,
                    ..
                } => {
                    handle_add_note(note);
                    preview_note.set(None);
                }
                AppMode::Edit {
                    edit_mode: EditMode::Remove,
                    ..
                } => {
                    handle_remove_note(note);
                    preview_note.set(None);
                }
                _ => {}
            }
        } else {
            match app_mode() {
                AppMode::View => {
                    if let Some((phantom_playhead_timestamp, ..)) = phantom_playhead() {
                        playback_state.set(PlaybackState {
                            playhead_timestamp: phantom_playhead_timestamp,
                        });
                    }
                }
                AppMode::Edit { .. } => {}
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
                // Staff lines
                g {
                    for line_number in 1..=5 {
                        line {
                            x1: "0",
                            y1: "{(2. * line_height) + (line_number as f32)*line_height}",
                            x2: "{width}",
                            y2: "{(2. * line_height) + (line_number as f32)*line_height}",
                        }
                    }
                }

                // Playhead
                if let Some(x) = playhead_timestamp
                    .and_then(|playhead_timestamp| {
                        let scales = scales();
                        let measure_scale = scales
                            .iter()
                            .enumerate()
                            .rev()
                            .find_map(|(i, scale)| {
                                measures
                                    .get(i)
                                    .and_then(|measure| {
                                        if measure.timestamp <= playhead_timestamp {
                                            Some(scale)
                                        } else {
                                            None
                                        }
                                    })
                            });
                        measure_scale.map(|measure_scale| measure_scale.get_x(playhead_timestamp))
                    })
                {
                    line {
                        stroke: "#8F00FF",
                        stroke_width: 1.5,

                        x1: "{x}",
                        y1: "0",
                        x2: "{x}",
                        y2: "{height}",
                    }
                }

                // Phantom playhead (only in View mode)
                if let Some((_,x)) = phantom_playhead() {
                    line {
                        stroke: "#8F00FF",
                        stroke_width: 1.5,
                        stroke_opacity: 0.25,

                        x1: "{x}",
                        y1: "0",
                        x2: "{x}",
                        y2: "{height}",
                    }
                }

                // Measures
                {
                    measures
                        .into_iter()
                        .enumerate()
                        .filter_map(|(i, measure)| {
                            let scale = scales.get(i)?;
                            let mouse_position = mouse_position()
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

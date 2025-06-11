use dioxus::prelude::*;

use crate::{
    data::{app_mode::AppMode, measure::Measure, position::Position},
    ui::{
        hooks::use_haptic,
        score::{StaffDimensions, measure_scale::MeasureScale},
    },
};

#[derive(PartialEq, Clone)]
pub struct PhantomPlayheadData {
    pub closest_timestamp: u32,
    pub x_pos: f32,
}

#[component]
pub fn PhantomPlayhead(data: PhantomPlayheadData, staff_dimensions: StaffDimensions) -> Element {
    let StaffDimensions { height, .. } = staff_dimensions;
    let PhantomPlayheadData {
        closest_timestamp,
        x_pos,
    } = data;
    let feedback_manager = use_haptic();

    use_effect(use_reactive(&closest_timestamp, move |_| {
        feedback_manager.trigger();
    }));

    rsx! {
        line {
            stroke: "#8F00FF",
            stroke_width: 1.5,
            stroke_opacity: 0.25,

            x1: "{x_pos}",
            y1: "0",
            x2: "{x_pos}",
            y2: "{height}",
        }
    }
}

pub fn use_phantom_playhead_data(
    app_mode: &Signal<AppMode>,
    mouse_position: &Option<Position>,
    scales: &Vec<MeasureScale>,
    measures: &Vec<Measure>,
) -> Option<PhantomPlayheadData> {
    let phantom_playhead = use_memo(use_reactive(
        (app_mode, mouse_position, scales, measures),
        |(app_mode, mouse_position, scales, measures)| {
            if matches!(app_mode(), AppMode::View) {
                if let Some(Position { x, .. }) = mouse_position {
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
                                    return Some(PhantomPlayheadData {
                                        closest_timestamp,
                                        x_pos: scale.get_x(closest_timestamp),
                                    });
                                }
                            }
                        }
                    }
                }
            }
            None
        },
    ));

    phantom_playhead()
}

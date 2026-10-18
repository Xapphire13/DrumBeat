use dioxus::prelude::*;

use crate::{
    data::{measure::Measure, position::Position},
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
    mouse_position: &Option<Position>,
    scales: &Vec<MeasureScale>,
    measures: &Vec<Measure>,
) -> Option<PhantomPlayheadData> {
    let phantom_playhead = use_memo(use_reactive(
        (mouse_position, scales, measures),
        |(mouse_position, scales, measures)| {
            if let Some(Position { x, .. }) = mouse_position {
                // Collect all note timestamps from this staff
                let mut note_timestamps = vec![];
                for measure in &measures {
                    for note in &measure.notes {
                        note_timestamps.push(note.timestamp);
                    }
                }

                if note_timestamps.is_empty() {
                    return None;
                }

                // Convert mouse x position to estimated timestamp. In the
                // margins, use the nearest end of the staff
                let mouse_timestamp = scales
                    .iter()
                    .find(|scale| x >= scale.outer_x_bounds.0 && x <= scale.outer_x_bounds.1)
                    .or_else(|| {
                        let first = scales.first()?;
                        if x < first.outer_x_bounds.0 {
                            Some(first)
                        } else {
                            scales.last()
                        }
                    })
                    .map_or(0, |scale| scale.get_timestamp(x));

                // Find closest actual note timestamp
                if let Some(&closest_timestamp) = note_timestamps
                    .iter()
                    .min_by_key(|&&ts| (i64::from(ts) - i64::from(mouse_timestamp)).abs())
                {
                    // Find which scale contains this timestamp and get x position
                    return scales
                        .iter()
                        .zip(&measures)
                        .find(|(_, measure)| measure.contains_timestamp(closest_timestamp))
                        .map(|(scale, _)| PhantomPlayheadData {
                            closest_timestamp,
                            x_pos: scale.get_x(closest_timestamp),
                        });
                }
            }
            None
        },
    ));

    phantom_playhead()
}

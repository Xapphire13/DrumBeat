use dioxus::prelude::*;

use crate::{
    data::measure::Measure,
    ui::{
        providers::playback_manager::use_playback_manager,
        score::{StaffDimensions, measure_scale::MeasureScale},
    },
};

#[component]
pub fn Playhead(
    staff_dimensions: StaffDimensions,
    measures: ReadSignal<Vec<Measure>>,
    scales: ReadSignal<Vec<MeasureScale>>,
) -> Element {
    let StaffDimensions { height, .. } = staff_dimensions;
    let playback_manager = use_playback_manager();
    let playhead_timestamp = playback_manager.playhead_timestamp();
    let playhead_x_pos = {
        let scales = scales();
        let measures = measures();
        measures
            .iter()
            .position(|measure| measure.contains_timestamp(playhead_timestamp))
            .and_then(|i| {
                let scale = scales.get(i)?;
                // Glide to where the next measure starts, or to the closing
                // bar line at the end of the staff
                let end_x = match (measures.get(i + 1), scales.get(i + 1)) {
                    (Some(next), Some(next_scale)) => next_scale.get_x(next.timestamp),
                    _ => scale.outer_x_bounds.1,
                };
                Some(scale.get_playhead_x(playhead_timestamp, end_x))
            })
    };

    rsx! {
        if let Some(x) = playhead_x_pos {
            line {
                stroke: "#8F00FF",
                stroke_width: 1.5,
                x1: "{x}",
                y1: "0",
                x2: "{x}",
                y2: "{height}",
            }
        }
    }
}

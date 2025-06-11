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
    measures: ReadOnlySignal<Vec<Measure>>,
    scales: ReadOnlySignal<Vec<MeasureScale>>,
) -> Element {
    let StaffDimensions { height, .. } = staff_dimensions;
    let playback_manager = use_playback_manager();
    let playhead_timestamp = playback_manager.playhead_timestamp();
    let playhead_x_pos = {
        let scales = scales();
        let measure_scale = scales.iter().enumerate().find_map(|(i, scale)| {
            measures.get(i).and_then(|measure| {
                if measure.contains_timestamp(playhead_timestamp) {
                    Some(scale)
                } else {
                    None
                }
            })
        });

        measure_scale.map(|measure_scale| measure_scale.get_x(playhead_timestamp))
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

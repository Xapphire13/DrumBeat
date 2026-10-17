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
    let is_playing = playback_manager.is_playing();
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
                // The playhead is redrawn when it moves to another staff, so
                // keep that staff in view while playing
                onmounted: move |event: MountedEvent| async move {
                    if is_playing {
                        let _ = event.scroll_to(ScrollBehavior::Smooth).await;
                    }
                },

                x1: "{x}",
                y1: "0",
                x2: "{x}",
                y2: "{height}",
            }
        }
    }
}

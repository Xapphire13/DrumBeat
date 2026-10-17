use dioxus::prelude::*;
use staff::Staff;

use crate::{
    data::measure::Measure,
    ui::{hooks::use_size, playback_bar::PlaybackBar, providers::score_manager::use_score_manager},
};

mod measure;
mod measure_scale;
mod note;
mod staff;

#[derive(PartialEq, Clone)]
pub struct StaffDimensions {
    width: f32,
    height: f32,
}

#[component]
pub fn Score() -> Element {
    let (width, _, handle_on_resize) = use_size();
    let score = use_score_manager();
    let measures_per_staff = ((width / 350.0).floor() as u32).max(1);
    let chunked_measures: Vec<Vec<Measure>> = score()
        .measures
        .chunks(measures_per_staff as usize)
        .map(<[Measure]>::to_vec)
        .collect();

    rsx! {
        div { class: "score", onresize: handle_on_resize,

            {chunked_measures.into_iter().map(|measures| rsx! {
                Staff {
                    dimensions: StaffDimensions {
                        width: width as f32,
                        height: 120.0,
                    },
                    measures,
                }
            })}

            PlaybackBar {}
        }
    }
}

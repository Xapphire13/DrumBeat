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

const STAFF_HEIGHT: f32 = 120.0;

#[derive(PartialEq, Clone)]
pub struct StaffDimensions {
    width: f32,
    height: f32,
}

/// The measures on one staff, with the width each one is drawn at
#[derive(PartialEq, Clone)]
pub struct StaffLayout {
    measures: Vec<Measure>,
    widths: Vec<f32>,
}

#[component]
pub fn Score() -> Element {
    let (width, _, handle_on_resize) = use_size();
    let score = use_score_manager();
    let width = width as f32;
    let staves = use_memo(use_reactive((&width,), move |(width,)| {
        layout_staves(&score().measures, width, staff::line_height(STAFF_HEIGHT))
    }));

    rsx! {
        div { class: "score", onresize: handle_on_resize,

            div { class: "staves",
                if score().measures.is_empty() {
                    div { class: "empty-score", "Open a Guitar Pro 8 file to get started" }
                }

                {staves().into_iter().map(|layout| rsx! {
                    Staff {
                        dimensions: StaffDimensions {
                            width,
                            height: STAFF_HEIGHT,
                        },
                        layout,
                    }
                })}
            }

            PlaybackBar {}
        }
    }
}

/// Stretching a measure past this many times its natural width looks sparse,
/// so a short last staff is left short instead
const MAX_LAST_STAFF_STRETCH: f32 = 1.5;

/// Break the measures into staves that fit `width`, giving each measure room
/// for its notes, then stretch each staff's measures to fill it
fn layout_staves(measures: &[Measure], width: f32, line_height: f32) -> Vec<StaffLayout> {
    if width <= 0.0 {
        return vec![];
    }

    let mut staves = vec![];
    let mut current = StaffLayout {
        measures: vec![],
        widths: vec![],
    };
    let mut current_width = 0.0;

    for measure in measures {
        let measure_width = measure_scale::natural_width(measure, line_height);
        if !current.measures.is_empty() && current_width + measure_width > width {
            staves.push(std::mem::replace(
                &mut current,
                StaffLayout {
                    measures: vec![],
                    widths: vec![],
                },
            ));
            current_width = 0.0;
        }
        current.measures.push(measure.clone());
        current.widths.push(measure_width);
        current_width += measure_width;
    }
    if !current.measures.is_empty() {
        staves.push(current);
    }

    let staff_count = staves.len();
    for (i, staff) in staves.iter_mut().enumerate() {
        let natural_total: f32 = staff.widths.iter().sum();
        let mut stretch = width / natural_total;
        if i + 1 == staff_count {
            stretch = stretch.min(MAX_LAST_STAFF_STRETCH);
        }
        for measure_width in &mut staff.widths {
            *measure_width *= stretch;
        }
    }

    staves
}

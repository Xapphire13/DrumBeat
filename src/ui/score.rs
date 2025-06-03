use dioxus::prelude::*;
use staff::Staff;

use crate::{
    data::{measure::Measure, note::Note, score::Score as ScoreData},
    ui::{
        hooks::use_size,
        playback_bar::PlaybackBar,
        providers::{playback_manager::use_playback_manager, score_manager::use_score_manager},
    },
};

mod measure;
mod measure_scale;
mod note;
mod preview_note;
mod staff;

#[component]
pub fn Score() -> Element {
    let (width, _, handle_on_resize) = use_size();
    let mut score = use_score_manager();
    let measures_per_staff = ((width / 350.0).floor() as u32).max(1);
    let playback_manager = use_playback_manager();
    let playhead_staff_number = use_memo(move || {
        let mut ticks = 0;
        for (staff_index, staff_measures) in score()
            .measures
            .chunks(measures_per_staff as usize)
            .enumerate()
        {
            ticks += staff_measures
                .iter()
                .map(|measure| measure.get_length())
                .sum::<u32>();

            if ticks > playback_manager.playhead_timestamp() {
                return staff_index as u32;
            }
        }

        0
    });
    let chunked_measures: Vec<Vec<Measure>> = score()
        .measures
        .chunks(measures_per_staff as usize)
        .map(|chunk| chunk.to_vec())
        .collect();

    let handle_remove_note = move |note: Note| {
        let updated_score = ScoreData {
            measures: score()
                .measures
                .into_iter()
                .map(|mut measure| {
                    if measure.timestamp <= note.timestamp
                        && note.timestamp - measure.timestamp < measure.get_length()
                    {
                        measure.remove_note(note.clone());
                    }

                    measure
                })
                .collect(),
        };

        score.set(updated_score);
    };

    let handle_add_note = move |note: Note| {
        let updated_score = ScoreData {
            measures: score()
                .measures
                .into_iter()
                .map(|mut measure| {
                    if measure.timestamp <= note.timestamp
                        && note.timestamp - measure.timestamp < measure.get_length()
                    {
                        measure.add_note(note.clone());
                    }

                    measure
                })
                .collect(),
        };

        score.set(updated_score);
    };

    rsx! {
        div { class: "score", onresize: handle_on_resize,

            {
                chunked_measures
                    .into_iter()
                    .enumerate()
                    .map(|(i, measures)| rsx! {
                        Staff {
                            width: width as f32,
                            height: 120.0,
                            measures,
                            handle_add_note,
                            handle_remove_note,
                            playhead_timestamp: if playhead_staff_number == i as u32 { Some(playback_manager.playhead_timestamp()) } else { None },
                        }
                    })
            }

            PlaybackBar {}
        }
    }
}

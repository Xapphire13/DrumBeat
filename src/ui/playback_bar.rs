use dioxus::prelude::*;
use dioxus_feather_icons::prelude::*;

use crate::ui::providers::{
    playback_manager::use_playback_manager, score_manager::use_score_manager,
};

#[component]
pub fn PlaybackBar() -> Element {
    let mut playback_manager = use_playback_manager();
    let score = use_score_manager();
    let measure_index = score()
        .measures
        .iter()
        .enumerate()
        .find(|(_, measure)| measure.timestamp > playback_manager.playhead_timestamp())
        .map(|(i, ..)| i.saturating_sub(1))
        .unwrap_or_else(|| score().measures.len().saturating_sub(1));
    let number_of_measures = score().measures.len() as u32;

    let note_timestamps = use_memo(move || {
        let mut timestamps = vec![];

        for measure in score().measures {
            for note in &measure.notes {
                if let Some(&last_timestamp) = timestamps.last() {
                    if last_timestamp < note.timestamp {
                        timestamps.push(note.timestamp);
                    }
                } else {
                    timestamps.push(note.timestamp);
                }
            }
        }

        timestamps
    });

    let handle_skip_back = move |_: MouseEvent| {
        let new_measure_index = measure_index.saturating_sub(1);
        let new_timestamp = score()
            .measures
            .get(new_measure_index)
            .map(|measure| measure.timestamp)
            .unwrap_or(0);

        playback_manager.update_playhead(new_timestamp);
    };

    let handle_skip_forward = move |_: MouseEvent| {
        let new_measure_index = measure_index.saturating_add(1);

        if let Some(new_timestamp) = score()
            .measures
            .get(new_measure_index)
            .map(|measure| measure.timestamp)
        {
            playback_manager.update_playhead(new_timestamp);
        }
    };

    let handle_prev_note = move |_: MouseEvent| {
        let timestamps = note_timestamps();
        if let Some(timestamp_index) = timestamps
            .iter()
            .position(|&ts| ts == playback_manager.playhead_timestamp())
        {
            if let Some(&next_timestamp) = timestamps.get(timestamp_index.saturating_sub(1)) {
                playback_manager.update_playhead(next_timestamp);
            }
        }
    };

    let handle_next_note = move |_: MouseEvent| {
        let timestamps = note_timestamps();
        if let Some(timestamp_index) = timestamps
            .iter()
            .position(|&ts| ts == playback_manager.playhead_timestamp())
        {
            if let Some(&next_timestamp) = timestamps.get(timestamp_index.saturating_add(1)) {
                playback_manager.update_playhead(next_timestamp);
            }
        }
    };

    rsx! {
        div { class: "playback-bar-container",

            div { class: "playback-bar",
                "Measure:"
                input {
                    class: "measure-number",
                    r#type: "text",
                    value: measure_index + 1,
                }
                "/ {number_of_measures}"
                button { onclick: handle_skip_back, {icon!(skip_back)} }
                button { onclick: handle_prev_note, {icon!(chevron_left)} }
                button { {icon!(play)} }
                button { onclick: handle_next_note, {icon!(chevron_right)} }
                button { onclick: handle_skip_forward, {icon!(skip_forward)} }
            }
        }
    }
}

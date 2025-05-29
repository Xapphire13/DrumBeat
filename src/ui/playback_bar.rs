use dioxus::prelude::*;
use dioxus_feather_icons::prelude::*;

use crate::ui::hooks::{PlaybackState, use_playback_state, use_score};

// TODO support time signatures other than 4/4
const TICKS_PER_MEASURE: u32 = 256;

#[component]
pub fn PlaybackBar() -> Element {
    let mut playback_state_ctx = use_playback_state();
    let playback_state = playback_state_ctx();
    let score_ctx = use_score();
    let score = score_ctx();
    let measure_number = playback_state.playhead_timestamp / TICKS_PER_MEASURE;
    let number_of_measures = score.measures.len() as u32;

    let note_timestamps = use_memo(use_reactive(&score, |score| {
        let mut timestamps = vec![];

        for measure in &score.measures {
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
    }));

    let handle_skip_back = move |_: MouseEvent| {
        let new_timestamp = TICKS_PER_MEASURE * measure_number.saturating_sub(1);

        playback_state_ctx.set(PlaybackState {
            playhead_timestamp: new_timestamp,
        });
    };

    let handle_skip_forward = move |_: MouseEvent| {
        let new_timestamp =
            TICKS_PER_MEASURE * measure_number.saturating_add(1).min(number_of_measures - 1);

        playback_state_ctx.set(PlaybackState {
            playhead_timestamp: new_timestamp,
        });
    };

    let handle_prev_note = move |_: MouseEvent| {
        let timestamps = note_timestamps();
        if let Some(timestamp_index) = timestamps
            .iter()
            .position(|&ts| ts == playback_state.playhead_timestamp)
        {
            if let Some(&next_timestamp) = timestamps.get(timestamp_index.saturating_sub(1)) {
                playback_state_ctx.set(PlaybackState {
                    playhead_timestamp: next_timestamp,
                })
            }
        }
    };

    let handle_next_note = move |_: MouseEvent| {
        let timestamps = note_timestamps();
        if let Some(timestamp_index) = timestamps
            .iter()
            .position(|&ts| ts == playback_state.playhead_timestamp)
        {
            if let Some(&next_timestamp) = timestamps.get(timestamp_index.saturating_add(1)) {
                playback_state_ctx.set(PlaybackState {
                    playhead_timestamp: next_timestamp,
                })
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
                    value: measure_number + 1,
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

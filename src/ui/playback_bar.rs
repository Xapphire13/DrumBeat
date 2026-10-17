use dioxus::prelude::*;
use dioxus_feather_icons::prelude::*;

use crate::ui::providers::{
    playback_manager::use_playback_manager, score_manager::use_score_manager,
};

#[component]
pub fn PlaybackBar() -> Element {
    let mut playback_manager = use_playback_manager();
    let score = use_score_manager();
    let measure_index = playback_manager.calculate_measure_index(&score());
    let number_of_measures = score().measures.len() as u32;

    let handle_skip_measure_back = move |_: MouseEvent| {
        let new_measure_index = measure_index.saturating_sub(1);
        playback_manager.skip_to_measure(new_measure_index, &score());
    };

    let handle_skip_measure_forward = move |_: MouseEvent| {
        let new_measure_index = measure_index.saturating_add(1);
        playback_manager.skip_to_measure(new_measure_index, &score());
    };

    let handle_prev_note = move |_: MouseEvent| playback_manager.skip_to_previous_note(&score());

    let handle_next_note = move |_: MouseEvent| playback_manager.skip_to_next_note(&score());

    let is_playing = playback_manager.is_playing();
    let handle_play_pause = move |_: MouseEvent| {
        if is_playing {
            playback_manager.pause();
        } else {
            playback_manager.play(&score());
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
                button { onclick: handle_skip_measure_back, {icon!(skip_back)} }
                button { onclick: handle_prev_note, {icon!(chevron_left)} }
                button { onclick: handle_play_pause,
                    if is_playing {
                        {icon!(pause)}
                    } else {
                        {icon!(play)}
                    }
                }
                button { onclick: handle_next_note, {icon!(chevron_right)} }
                button { onclick: handle_skip_measure_forward, {icon!(skip_forward)} }
            }
        }
    }
}

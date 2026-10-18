use dioxus::prelude::*;
use dioxus_feather_icons::prelude::*;

use crate::ui::providers::{
    playback_manager::use_playback_manager, score_manager::use_score_manager,
};

/// The range of the speed slider. Typing a BPM can go beyond it
const SLIDER_MIN_PERCENT: f64 = 25.0;
const SLIDER_MAX_PERCENT: f64 = 200.0;

#[component]
pub fn PlaybackBar() -> Element {
    let mut playback_manager = use_playback_manager();
    let score = use_score_manager();
    let performance_timestamp = playback_manager.performance_timestamp();
    // Measures are counted as they're played, with repeats expanded
    let (played_index, number_of_measures, (pass, passes)) = {
        let performance = &score.read().performance;
        let played_index = performance.played_measure_index(performance_timestamp);
        (
            played_index,
            performance.measures().len(),
            performance.pass(played_index),
        )
    };

    let speed = playback_manager.speed();
    let speed_percent = (speed * 100.0).round();
    let score_bpm = score()
        .performance
        .tempo_map()
        .bpm_at(performance_timestamp);
    let bpm = (score_bpm * speed).round();

    let handle_skip_measure_back =
        move |_: MouseEvent| playback_manager.skip_measures(-1, &score());

    let handle_skip_measure_forward =
        move |_: MouseEvent| playback_manager.skip_measures(1, &score());

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

    let handle_speed_input = move |event: FormEvent| {
        if let Ok(percent) = event.value().parse::<f64>() {
            playback_manager.set_speed(percent / 100.0);
        }
    };

    // Typing a BPM sets the speed that plays the current tempo at it; any
    // tempo changes in the score scale with it
    let handle_bpm_change = move |event: FormEvent| {
        if let Ok(bpm) = event.value().parse::<f64>()
            && bpm > 0.0
        {
            playback_manager.set_speed(bpm / score_bpm);
        }
    };

    let handle_reset_speed = move |_: MouseEvent| playback_manager.set_speed(1.0);

    rsx! {
        div { class: "playback-bar-container",

            div { class: "playback-bar",
                "Measure:"
                input {
                    class: "measure-number",
                    r#type: "text",
                    value: played_index + 1,
                }
                "/ {number_of_measures}"
                if passes > 1 {
                    span { class: "repeat-pass", title: "Time through the repeat", "×{pass}/{passes}" }
                }
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

                div { class: "tempo-control", title: "Playback speed, relative to the score's tempo",
                    "Tempo"
                    input {
                        r#type: "range",
                        min: SLIDER_MIN_PERCENT,
                        max: SLIDER_MAX_PERCENT,
                        step: 5,
                        value: speed_percent,
                        oninput: handle_speed_input,
                    }
                    button {
                        class: "tempo-reset",
                        title: "Reset to the score's tempo",
                        disabled: speed_percent == 100.0,
                        onclick: handle_reset_speed,
                        "{speed_percent}%"
                    }
                    span { class: "tempo-bpm",
                        "♩ ="
                        input {
                            class: "bpm-input",
                            r#type: "number",
                            title: "Play at an exact tempo",
                            min: 1,
                            value: bpm,
                            onchange: handle_bpm_change,
                        }
                    }
                }
            }
        }
    }
}

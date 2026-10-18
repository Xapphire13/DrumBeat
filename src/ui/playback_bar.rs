use std::cmp::Ordering;

use dioxus::prelude::*;
use lucide_dioxus::{
    ChevronLeft, ChevronRight, Pause, Play, Repeat, RotateCcw, SkipBack, SkipForward, Volume1,
    Volume2, VolumeX,
};

use crate::{
    data::performance::PlayedMeasure,
    ui::providers::{playback_manager::use_playback_manager, score_manager::use_score_manager},
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
    let (played_index, number_of_measures, (pass, passes), segments) = {
        let performance = &score.read().performance;
        let played_index = performance.played_measure_index(performance_timestamp);
        (
            played_index,
            performance.measures().len(),
            performance.pass(played_index),
            measure_segments(performance.measures(), played_index),
        )
    };

    let speed = playback_manager.speed();
    let speed_percent = (speed * 100.0).round();
    // How much of the slider's track is filled, as a percentage
    let slider_fill =
        ((speed_percent - SLIDER_MIN_PERCENT) / (SLIDER_MAX_PERCENT - SLIDER_MIN_PERCENT) * 100.0)
            .clamp(0.0, 100.0);
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

    let is_muted = playback_manager.is_muted();
    // A muted slider reads as empty, and dragging it unmutes
    let volume_percent = if is_muted {
        0.0
    } else {
        (playback_manager.volume() * 100.0).round()
    };
    let handle_toggle_mute = move |_: MouseEvent| playback_manager.toggle_mute();
    let handle_volume_input = move |event: FormEvent| {
        if let Ok(percent) = event.value().parse::<f64>() {
            playback_manager.set_volume(percent / 100.0);
        }
    };

    rsx! {
        div { class: "playback-bar",
            div { class: "measure-strip", "aria-hidden": "true",
                for group in segments {
                    if let Some(class) = group.pass_class {
                        span {
                            class,
                            style: "flex: {group.segments.len()} 1 0",
                            for &class in &group.segments {
                                span { class }
                            }
                        }
                    } else {
                        for class in group.segments {
                            span { class }
                        }
                    }
                }
            }

            div { class: "playback-controls",
                div { class: "playback-position",
                    div { class: "measure-counter",
                        span { class: "control-label", "Measure" }
                        div { class: "measure-count",
                            span { class: "measure-current", "{played_index + 1}" }
                            span { class: "measure-total", "/ {number_of_measures}" }
                        }
                    }
                    if passes > 1 {
                        span {
                            class: "repeat-pass",
                            title: "Time through the repeat",
                            Repeat { size: 13 }
                            span { class: "repeat-pass-long", "Pass {pass} of {passes}" }
                            span { class: "repeat-pass-short", "{pass}/{passes}" }
                        }
                    }
                }

                div { class: "transport",
                    button {
                        class: "transport-button",
                        title: "Previous measure",
                        onclick: handle_skip_measure_back,
                        SkipBack { size: 20 }
                    }
                    button {
                        class: "transport-button",
                        title: "Previous note",
                        onclick: handle_prev_note,
                        ChevronLeft { size: 22 }
                    }
                    button {
                        class: "play-button",
                        title: if is_playing { "Pause" } else { "Play" },
                        onclick: handle_play_pause,
                        if is_playing {
                            Pause { size: 22, fill: "currentColor" }
                        } else {
                            Play { size: 22, fill: "currentColor" }
                        }
                    }
                    button {
                        class: "transport-button",
                        title: "Next note",
                        onclick: handle_next_note,
                        ChevronRight { size: 22 }
                    }
                    button {
                        class: "transport-button",
                        title: "Next measure",
                        onclick: handle_skip_measure_forward,
                        SkipForward { size: 20 }
                    }
                }

                div { class: "playback-settings",
                    div { class: "volume-control",
                        button {
                            class: "mute-button",
                            title: if is_muted { "Unmute (M)" } else { "Mute (M)" },
                            onclick: handle_toggle_mute,
                            if volume_percent == 0.0 {
                                VolumeX { size: 18 }
                            } else if volume_percent < 50.0 {
                                Volume1 { size: 18 }
                            } else {
                                Volume2 { size: 18 }
                            }
                        }
                        input {
                            r#type: "range",
                            title: "Volume",
                            style: "--fill: {volume_percent}%",
                            min: 0,
                            max: 100,
                            step: 1,
                            value: volume_percent,
                            oninput: handle_volume_input,
                        }
                    }

                    div {
                        class: "tempo-control",
                        title: "Playback speed, relative to the score's tempo",
                        label { class: "tempo-slider",
                            span { class: "tempo-slider-labels",
                                span { class: "control-label", "Tempo" }
                                span { "{SLIDER_MIN_PERCENT}–{SLIDER_MAX_PERCENT}%" }
                            }
                            input {
                                r#type: "range",
                                style: "--fill: {slider_fill}%",
                                min: SLIDER_MIN_PERCENT,
                                max: SLIDER_MAX_PERCENT,
                                step: 5,
                                value: speed_percent,
                                oninput: handle_speed_input,
                            }
                        }
                        button {
                            class: "tempo-reset",
                            title: "Reset to the score's tempo",
                            disabled: speed_percent == 100.0,
                            onclick: handle_reset_speed,
                            "{speed_percent}%"
                            RotateCcw { size: 13 }
                        }
                        label { class: "bpm-field",
                            input {
                                r#type: "number",
                                title: "Play at an exact tempo",
                                min: 1,
                                value: bpm,
                                onchange: handle_bpm_change,
                            }
                            "BPM"
                        }
                    }
                }
            }
        }
    }
}

/// A run of segments in the measure strip. Each pass through a repeat is
/// its own group, drawn with a bracket above it
#[derive(Debug, PartialEq)]
struct StripGroup {
    /// The bracket's class, for a pass through a repeat
    pass_class: Option<&'static str>,
    /// The class of each played measure's segment
    segments: Vec<&'static str>,
}

/// The measure strip's segments, marking where the playhead is, grouped so
/// each pass through a repeat can be bracketed
fn measure_segments(measures: &[PlayedMeasure], played_index: usize) -> Vec<StripGroup> {
    let written_measures = measures
        .iter()
        .map(|m| m.measure_index + 1)
        .max()
        .unwrap_or(0);
    let mut times_played = vec![0; written_measures];
    for measure in measures {
        times_played[measure.measure_index] += 1;
    }

    // Measures that playback jumps back from, which end each pass
    let mut ends_pass = vec![false; written_measures];
    for pair in measures.windows(2) {
        if pair[1].measure_index <= pair[0].measure_index {
            ends_pass[pair[0].measure_index] = true;
        }
    }

    let mut groups: Vec<(bool, Vec<usize>)> = vec![];
    for (i, measure) in measures.iter().enumerate() {
        let repeated = times_played[measure.measure_index] > 1;
        let continues_pass = i > 0
            && !ends_pass[measures[i - 1].measure_index]
            && groups.last().is_some_and(|(in_pass, _)| *in_pass);
        match groups.last_mut() {
            Some((_, indices)) if repeated && continues_pass => indices.push(i),
            _ => groups.push((repeated, vec![i])),
        }
    }

    groups
        .into_iter()
        .map(|(in_pass, indices)| {
            let pass_class = in_pass.then(|| {
                let (first, last) = (indices[0], indices[indices.len() - 1]);
                if last < played_index {
                    "repeat-pass-group done"
                } else if first <= played_index {
                    "repeat-pass-group active"
                } else {
                    "repeat-pass-group"
                }
            });
            let segments = indices
                .into_iter()
                .map(|i| match i.cmp(&played_index) {
                    Ordering::Less => "measure-segment played",
                    Ordering::Equal => "measure-segment current",
                    Ordering::Greater => "measure-segment",
                })
                .collect();
            StripGroup {
                pass_class,
                segments,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn played(measure_indices: &[usize]) -> Vec<PlayedMeasure> {
        measure_indices
            .iter()
            .map(|&measure_index| PlayedMeasure {
                measure_index,
                timestamp: 0,
                written_timestamp: 0,
                length: 1,
            })
            .collect()
    }

    #[test]
    fn brackets_each_pass_of_a_repeat() {
        // Measure 0, then 1–2 played three times, then 3
        let measures = played(&[0, 1, 2, 1, 2, 1, 2, 3]);
        let groups = measure_segments(&measures, 3);
        let pass_classes: Vec<_> = groups.iter().map(|g| g.pass_class).collect();
        assert_eq!(
            pass_classes,
            [
                None,
                Some("repeat-pass-group done"),
                Some("repeat-pass-group active"),
                Some("repeat-pass-group"),
                None,
            ]
        );
        let lengths: Vec<_> = groups.iter().map(|g| g.segments.len()).collect();
        assert_eq!(lengths, [1, 2, 2, 2, 1]);
        assert_eq!(
            groups[2].segments,
            ["measure-segment current", "measure-segment"]
        );
    }

    #[test]
    fn back_to_back_repeats_are_separate_passes() {
        // 0–1 played twice, then 2–3 played twice
        let measures = played(&[0, 1, 0, 1, 2, 3, 2, 3]);
        let lengths: Vec<_> = measure_segments(&measures, 0)
            .iter()
            .map(|g| g.segments.len())
            .collect();
        assert_eq!(lengths, [2, 2, 2, 2]);
    }
}

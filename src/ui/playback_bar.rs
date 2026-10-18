use std::cmp::Ordering;

use dioxus::{html::input_data::MouseButton, prelude::*};
use lucide_dioxus::{
    ChevronLeft, ChevronRight, ChevronUp, Gauge, Infinity, Pause, Play, Repeat, RotateCcw,
    SkipBack, SkipForward, Volume1, Volume2, VolumeX, X,
};

use crate::{
    data::{
        loop_range::{LoopRange, count_label},
        performance::PlayedMeasure,
    },
    ui::providers::{
        loop_editor::{LoopDrag, use_loop_editor},
        playback_manager::use_playback_manager,
        score_manager::use_score_manager,
    },
};

/// The range of the speed slider. Typing a BPM can go beyond it
const SLIDER_MIN_PERCENT: f64 = 25.0;
const SLIDER_MAX_PERCENT: f64 = 200.0;

#[component]
pub fn PlaybackBar() -> Element {
    let mut playback_manager = use_playback_manager();
    let score = use_score_manager();
    let loop_editor = use_loop_editor();
    let performance_timestamp = playback_manager.performance_timestamp();
    // The loop as it's being dragged out, or else the one playing
    let shown_loop = loop_editor.shown(&playback_manager, &score.read());
    let loop_bands: Vec<Option<LoopBand>> = {
        let performance = &score.read().performance;
        (0..performance.measures().len())
            .map(|index| shown_loop.and_then(|range| LoopBand::new(range, performance, index)))
            .collect()
    };
    let loop_chip = shown_loop.map(|range| {
        let score = score.read();
        let label = |timestamp| count_label(timestamp, &score.performance, &score.measures);
        let (start, end) = (label(range.start), label(range.end));
        let length = format_beats(range.beats(&score.performance, &score.measures));
        (
            format!("{start}–{end}"),
            format!("Looping {start} up to {end}, {length}"),
        )
    });
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
    // Wall-clock time at the current speed, through every repeat. Tempo
    // changes are already in the performance's tempo map
    let (elapsed_seconds, total_seconds) = {
        let performance = &score.read().performance;
        let tempo_map = performance.tempo_map();
        (
            tempo_map.ticks_to_seconds(performance_timestamp) / speed,
            tempo_map.ticks_to_seconds(performance.end_timestamp()) / speed,
        )
    };
    let mut show_remaining = use_signal(|| false);
    let (end_time, end_time_title) = if show_remaining() {
        (
            format!("−{}", format_time(total_seconds - elapsed_seconds)),
            "Time remaining. Click to show the total",
        )
    } else {
        (
            format_time(total_seconds),
            "Total time, with repeats. Click to show the time remaining",
        )
    };
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

    // Tempo is a chip showing the speed and BPM, with the controls in a
    // popover above it
    let mut tempo_open = use_signal(|| false);
    let tempo_chip_class = format!(
        "tempo-chip{}{}",
        if tempo_open() { " open" } else { "" },
        if speed_percent == 100.0 {
            ""
        } else {
            " off-tempo"
        },
    );

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
            div { class: "timeline",
            span { class: "timeline-time", title: "Time played", "{format_time(elapsed_seconds)}" }
            div { class: "measure-strip", "aria-hidden": "true",
                for group in segments {
                    if let Some(class) = group.pass_class {
                        span {
                            class,
                            style: "flex: {group.segments.len()} 1 0",
                            for (offset , & class) in group.segments.iter().enumerate() {
                                StripSegment {
                                    index: group.first_index + offset,
                                    class,
                                    band: loop_bands[group.first_index + offset],
                                }
                            }
                        }
                    } else {
                        for (offset , & class) in group.segments.iter().enumerate() {
                            StripSegment {
                                index: group.first_index + offset,
                                class,
                                band: loop_bands[group.first_index + offset],
                            }
                        }
                    }
                }
            }
            button {
                class: "timeline-time timeline-end",
                title: end_time_title,
                onclick: move |_| show_remaining.toggle(),
                "{end_time}"
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
                    div { class: "position-chips",
                        if passes > 1 {
                            span {
                                class: "repeat-pass",
                                title: "Time through the repeat",
                                Repeat { size: 13 }
                                span { class: "repeat-pass-long", "Pass {pass} of {passes}" }
                                span { class: "repeat-pass-short", "{pass}/{passes}" }
                            }
                        }
                        if let Some((range, description)) = loop_chip {
                            span {
                                class: "loop-chip",
                                title: "{description}",
                                aria_label: "{description}",
                                Infinity { size: 15 }
                                span { class: "loop-chip-range", "{range}" }
                                button {
                                    class: "loop-clear",
                                    title: "Stop looping (Esc)",
                                    aria_label: "Stop looping",
                                    onclick: move |_| playback_manager.clear_loop(),
                                    X { size: 12 }
                                }
                            }
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
                        label { class: "labelled-slider volume-slider",
                            span { class: "slider-labels",
                                span { class: "control-label", "Volume" }
                                span { "{volume_percent}%" }
                            }
                            input {
                                r#type: "range",
                                style: "--fill: {volume_percent}%",
                                min: 0,
                                max: 100,
                                step: 1,
                                value: volume_percent,
                                oninput: handle_volume_input,
                            }
                        }
                    }

                    div { class: "tempo-control",
                        button {
                            class: "{tempo_chip_class}",
                            title: "Playback speed, relative to the score's tempo",
                            aria_haspopup: "dialog",
                            aria_expanded: tempo_open(),
                            onclick: move |_| tempo_open.set(!tempo_open()),
                            Gauge { size: 16 }
                            span { class: "tempo-chip-speed", "{speed_percent}%" }
                            span { class: "tempo-chip-bpm", "{bpm} BPM" }
                            ChevronUp { size: 14 }
                        }
                        if tempo_open() {
                            // Clicking anywhere outside the popover closes it
                            div {
                                class: "popover-backdrop",
                                onclick: move |_| tempo_open.set(false),
                            }
                            div {
                                class: "tempo-popover",
                                role: "dialog",
                                aria_label: "Tempo",
                                onkeydown: move |event: KeyboardEvent| {
                                    if event.key().to_string() == "Escape" {
                                        tempo_open.set(false);
                                    }
                                },
                                label { class: "labelled-slider tempo-slider",
                                    span { class: "slider-labels",
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
                                div { class: "tempo-popover-row",
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
        }
    }
}

/// Seconds as m:ss, or h:mm:ss from an hour up
fn format_time(seconds: f64) -> String {
    // Truncated, so the elapsed time ticks over on the second
    let seconds = seconds.max(0.0) as u64;
    let (hours, minutes, seconds) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

/// One played measure in the measure strip. Pressing on it and dragging
/// across others loops whole measures; a click without dragging moves the
/// playhead to the start of the measure, on the pass through a repeat that
/// the segment stands for
#[component]
fn StripSegment(index: usize, class: &'static str, band: Option<LoopBand>) -> Element {
    let mut loop_editor = use_loop_editor();

    rsx! {
        span {
            class: "measure-slot",
            onmousedown: move |event: MouseEvent| {
                if event.trigger_button() == Some(MouseButton::Primary) {
                    loop_editor.start_drag(LoopDrag::Measures {
                        anchor: index,
                        current: index,
                        moved: false,
                    });
                }
            },
            onmouseenter: move |_| loop_editor.drag_to_measure(index),
            if let Some(band) = band {
                span { class: "loop-band", style: band.style() }
            }
            span { class }
        }
    }
}

/// The part of a measure strip segment the loop covers
#[derive(Clone, Copy, PartialEq, Debug)]
struct LoopBand {
    /// From the start of the segment, as fractions of it
    from: f64,
    to: f64,
    /// Whether the loop starts or ends in this measure, so the band's end is
    /// rounded off there instead of running on into the next segment
    starts: bool,
    ends: bool,
}

impl LoopBand {
    fn new(
        range: LoopRange,
        performance: &crate::data::performance::Performance,
        index: usize,
    ) -> Option<Self> {
        let (from, to) = range.measure_coverage(performance, index)?;
        let played = performance.measures()[index];
        Some(Self {
            from,
            to,
            starts: range.start >= played.timestamp,
            ends: range.end <= played.timestamp + played.length,
        })
    }

    fn style(self) -> String {
        let radius = |rounded: bool| if rounded { "3px" } else { "0" };
        let (start, end) = (radius(self.starts), radius(self.ends));
        // Bridge the gap to the next segment while the loop carries on
        let bridge = if self.ends { 0 } else { 2 };
        format!(
            "left: {}%; width: calc({}% + {bridge}px); border-radius: {start} {end} {end} {start}",
            self.from * 100.0,
            (self.to - self.from) * 100.0,
        )
    }
}

/// A loop's length, e.g. "15 beats", "½ beat" or "3¼ beats"
fn format_beats(beats: f64) -> String {
    let whole = beats.floor();
    let fraction = match ((beats - whole) * 12.0).round() as u32 {
        0 => "",
        3 => "¼",
        4 => "⅓",
        6 => "½",
        8 => "⅔",
        9 => "¾",
        12 => return format_beats(whole + 1.0),
        _ => return format!("{beats:.1} beats"),
    };
    let number = if whole == 0.0 && !fraction.is_empty() {
        fraction.to_string()
    } else {
        format!("{whole}{fraction}")
    };
    let unit = if beats <= 1.0 { "beat" } else { "beats" };
    format!("{number} {unit}")
}

/// A run of segments in the measure strip. Each pass through a repeat is
/// its own group, drawn with a bracket above it
#[derive(Debug, PartialEq)]
struct StripGroup {
    /// Index into the played measures of the group's first segment
    first_index: usize,
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
            let first_index = indices[0];
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
                first_index,
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
    fn formats_times() {
        assert_eq!(format_time(0.0), "0:00");
        assert_eq!(format_time(59.9), "0:59");
        assert_eq!(format_time(127.4), "2:07");
        assert_eq!(format_time(3725.0), "1:02:05");
        assert_eq!(format_time(-1.0), "0:00");
    }

    #[test]
    fn formats_loop_lengths() {
        assert_eq!(format_beats(15.0), "15 beats");
        assert_eq!(format_beats(1.0), "1 beat");
        assert_eq!(format_beats(0.5), "½ beat");
        assert_eq!(format_beats(14.5), "14½ beats");
        assert_eq!(format_beats(3.25), "3¼ beats");
        assert_eq!(format_beats(2.0 / 3.0), "⅔ beat");
        assert_eq!(format_beats(1.2), "1.2 beats");
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
        let first_indices: Vec<_> = groups.iter().map(|g| g.first_index).collect();
        assert_eq!(first_indices, [0, 1, 3, 5, 7]);
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

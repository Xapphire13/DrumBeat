use dioxus::prelude::*;
use lucide_dioxus::{ChevronDown, Metronome};

use crate::{
    audio::{ClickSound, MetronomeSettings, Subdivision},
    data::time_signature::TimeSignature,
    ui::{
        controls::{SegmentOption, Segmented, Switch},
        providers::{playback_manager::use_playback_manager, score_manager::use_score_manager},
    },
};

/// The click sounds offered while the score plays
const SOUNDS: [ClickSound; 3] = [ClickSound::Click, ClickSound::Wood, ClickSound::Rim];

/// Bars of count-in offered
const COUNT_INS: [(u32, &str); 3] = [(0, "Off"), (1, "1 bar"), (2, "2 bars")];

/// Pip sizes and the gap between them, to size the row as it slides in
const FIRST_PIP: u32 = 7;
const PIP: u32 = 5;
const PIP_GAP: u32 = 3;

/// The metronome that clicks along with the score: a toggle, which shows a
/// pip for each beat of the measure while it's on, and a menu of settings
#[component]
pub fn MetronomeControl() -> Element {
    let mut playback_manager = use_playback_manager();
    let score = use_score_manager();
    let settings = playback_manager.metronome();
    let mut open = use_signal(|| false);

    let timestamp = playback_manager.performance_timestamp();
    let (time_signature, bpm) = {
        let score = score.read();
        let performance = &score.performance;
        let time_signature = performance
            .measures()
            .get(performance.played_measure_index(timestamp))
            .map(|played| score.measures[played.measure_index].time_signature.clone());
        let bpm = performance.tempo_map().bpm_at(timestamp) * playback_manager.speed();
        (time_signature, bpm.round())
    };
    let time_signature = time_signature.unwrap_or(TimeSignature {
        notes_per_measure: 4,
        note_value: crate::data::note::NoteValue::Quarter,
    });
    let beats = time_signature.notes_per_measure;
    // Lit from the click that was heard, counting in too
    let lit_beat = playback_manager
        .click()
        .filter(|_| settings.enabled)
        .map(|click| click.beat);
    let pips_width = if settings.enabled {
        FIRST_PIP + (PIP + PIP_GAP) * beats.saturating_sub(1)
    } else {
        0
    };

    let mut update = move |change: &dyn Fn(&mut MetronomeSettings)| {
        let mut settings = playback_manager.metronome();
        change(&mut settings);
        playback_manager.set_metronome(settings, &score.peek());
    };

    rsx! {
        div { class: "metronome-control",
            div { class: if settings.enabled { "click-split on" } else { "click-split" },
                button {
                    class: "click-toggle",
                    title: if settings.enabled { "Turn the click off (K)" } else { "Click along with the score (K)" },
                    aria_label: "Metronome click",
                    aria_pressed: settings.enabled,
                    onclick: move |_| playback_manager.toggle_metronome(&score.peek()),
                    Metronome { size: 16 }
                    span {
                        class: "click-pips",
                        "aria-hidden": "true",
                        style: "width: {pips_width}px",
                        for beat in 0..beats {
                            span {
                                key: "{beat}",
                                class: pip_class(beat, lit_beat),
                            }
                        }
                    }
                }
                button {
                    class: "click-menu",
                    title: "Metronome settings",
                    aria_label: "Metronome settings",
                    aria_haspopup: "dialog",
                    aria_expanded: open(),
                    onclick: move |_| open.toggle(),
                    ChevronDown { size: 14, class: "popover-chevron" }
                }
            }
            if open() {
                div {
                    class: "toolbar-popover-backdrop",
                    onclick: move |_| open.set(false),
                }
                div {
                    class: "metronome-popover",
                    role: "dialog",
                    aria_label: "Metronome",
                    onkeydown: move |event: KeyboardEvent| {
                        if event.key().to_string() == "Escape" {
                            open.set(false);
                        }
                    },
                    div { class: "popover-header",
                        div { class: "popover-heading",
                            span { class: "control-label", "Metronome" }
                            span { class: "popover-subtitle",
                                "Follows playback · {bpm} BPM, {time_signature.label()}"
                            }
                        }
                        Switch {
                            checked: settings.enabled,
                            label: "Metronome click",
                            onchange: move |on| update(&|s| s.enabled = on),
                        }
                    }
                    div { class: "popover-field",
                        span { class: "control-label", "Sound" }
                        Segmented {
                            label: "Sound",
                            options: SOUNDS
                                .iter()
                                .map(|&sound| SegmentOption::new(sound.label(), settings.sound == sound))
                                .collect::<Vec<_>>(),
                            onselect: move |i: usize| update(&|s| s.sound = SOUNDS[i]),
                        }
                    }
                    label { class: "labelled-slider",
                        span { class: "slider-labels",
                            span { class: "control-label", "Volume" }
                            span { "{(settings.volume * 100.0).round()}%" }
                        }
                        input {
                            r#type: "range",
                            style: "--fill: {settings.volume * 100.0}%",
                            min: 0,
                            max: 100,
                            step: 1,
                            value: (settings.volume * 100.0).round(),
                            oninput: move |event: FormEvent| {
                                if let Ok(percent) = event.value().parse::<f64>() {
                                    update(&|s| s.volume = percent / 100.0);
                                }
                            },
                        }
                    }
                    div { class: "popover-field",
                        span { class: "control-label", "Beats" }
                        Segmented {
                            label: "Beats",
                            options: Subdivision::ALL
                                .iter()
                                .map(|&sub| SegmentOption::new(sub.label(), settings.subdivision == sub))
                                .collect::<Vec<_>>(),
                            onselect: move |i: usize| update(&|s| s.subdivision = Subdivision::ALL[i]),
                        }
                    }
                    div { class: "popover-field",
                        span { class: "control-label", "Count-in" }
                        Segmented {
                            label: "Count-in",
                            options: COUNT_INS
                                .iter()
                                .map(|&(bars, label)| SegmentOption::new(label, settings.count_in_bars == bars))
                                .collect::<Vec<_>>(),
                            onselect: move |i: usize| update(&|s| s.count_in_bars = COUNT_INS[i].0),
                        }
                    }
                    label { class: "checkbox-row",
                        input {
                            r#type: "checkbox",
                            checked: settings.accent_first,
                            onchange: move |event: FormEvent| {
                                let checked = event.checked();
                                update(&|s| s.accent_first = checked);
                            },
                        }
                        "Accent the first beat of each measure"
                    }
                }
            }
        }
    }
}

fn pip_class(beat: u32, lit_beat: Option<u32>) -> &'static str {
    match (beat == 0, lit_beat == Some(beat)) {
        (true, true) => "pip first lit",
        (true, false) => "pip first",
        (false, true) => "pip lit",
        (false, false) => "pip",
    }
}

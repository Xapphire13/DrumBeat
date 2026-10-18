use std::time::{Duration, Instant};

use dioxus::prelude::*;
use lucide_dioxus::{Minus, Play, Plus, Square};

use crate::{
    audio::{BeatKind, ClickPosition, ClickSound, SILENT_BAR_EVERY, Subdivision},
    ui::{
        controls::{SegmentOption, Segmented, Switch},
        providers::practice::{MAX_BPM, MIN_BPM, Meter, use_practice},
    },
};

const SOUNDS: [ClickSound; 4] = [
    ClickSound::Click,
    ClickSound::Wood,
    ClickSound::Rim,
    ClickSound::Cowbell,
];

/// Only taps this recent count towards the tempo, so it follows a change of
/// pace quickly
const TAP_WINDOW: Duration = Duration::from_millis(2500);

/// The most taps averaged, latest first
const MAX_TAPS: usize = 5;

/// The tempo slider moves in steps this big; − and + fine-tune it
const SLIDER_STEP: u32 = 5;

/// The standalone metronome: the beat and tempo on a stage, with its
/// settings in a panel beside it
#[component]
pub fn PracticeTool() -> Element {
    rsx! {
        div { class: "practice",
            Stage {}
            SidePanel {}
        }
    }
}

#[component]
fn Stage() -> Element {
    let mut practice = use_practice();
    let options = practice.options();
    let running = practice.is_running();
    let click = practice.click().filter(|_| running);
    let bpm = practice.bpm();
    let per_beat = options.subdivision.clicks_per_beat();
    let silent_bar = click.is_some_and(|click| click.silent);

    // Taps within the window of each other, latest last
    let mut taps = use_signal(Vec::<Instant>::new);
    // On press rather than release, which is when the beat is felt
    let handle_tap = move |_: MouseEvent| {
        let now = Instant::now();
        let mut taps = taps.write();
        taps.retain(|&tap| now.duration_since(tap) <= TAP_WINDOW);
        taps.push(now);
        let excess = taps.len().saturating_sub(MAX_TAPS);
        taps.drain(..excess);
        if let Some(bpm) = tapped_bpm(&taps) {
            practice.set_bpm(bpm);
        }
    };

    let caption = match click {
        Some(_) if silent_bar => "Silent bar · keep time".to_string(),
        Some(click) => format!("Bar {}", click.bar),
        None => "Click a beat to accent it, play it normally or silence it".to_string(),
    };

    // The stage flashes as an accented first beat is heard
    let flash = options.flash
        && !silent_bar
        && options.beats.first() == Some(&BeatKind::Accent)
        && click.is_some_and(|click| click.beat == 0 && click.sub == 0);

    let slider_fill = f64::from(bpm - MIN_BPM) / f64::from(MAX_BPM - MIN_BPM) * 100.0;

    rsx! {
        div { class: "practice-stage",
            div { class: if flash { "stage-flash on" } else { "stage-flash" } }
            div { class: "beat-row",
                for (beat , & kind) in options.beats.iter().enumerate() {
                    button {
                        key: "beat-{beat}",
                        class: beat_class(kind, beat, click),
                        title: match kind {
                            BeatKind::Accent => "Accented. Click to play it normally",
                            BeatKind::Normal => "Played normally. Click to silence it",
                            BeatKind::Silent => "Silent. Click to accent it",
                        },
                        onclick: move |_| practice.cycle_beat(beat),
                        "{beat + 1}"
                    }
                    for sub in 1..per_beat {
                        span {
                            key: "sub-{beat}-{sub}",
                            class: if click.is_some_and(|c| c.beat as usize == beat && c.sub == sub) { "sub-dot lit" } else { "sub-dot" },
                        }
                    }
                }
            }
            p { class: "stage-caption", "{caption}" }

            div { class: "bpm-row",
                button {
                    class: "bpm-step",
                    title: "Slower",
                    aria_label: "Slower",
                    disabled: bpm <= MIN_BPM,
                    onclick: move |_| practice.set_bpm(bpm.saturating_sub(1)),
                    Minus { size: 20 }
                }
                div { class: "bpm-readout",
                    span { class: "bpm-value", "{bpm}" }
                    span { class: "control-label", "BPM" }
                }
                button {
                    class: "bpm-step",
                    title: "Faster",
                    aria_label: "Faster",
                    disabled: bpm >= MAX_BPM,
                    onclick: move |_| practice.set_bpm(bpm + 1),
                    Plus { size: 20 }
                }
            }
            label { class: "labelled-slider practice-bpm-slider",
                span { class: "slider-labels",
                    span { class: "control-label", "Tempo" }
                    span { "{MIN_BPM}–{MAX_BPM}" }
                }
                input {
                    r#type: "range",
                    style: "--fill: {slider_fill}%",
                    min: MIN_BPM,
                    max: MAX_BPM,
                    step: SLIDER_STEP,
                    value: bpm,
                    oninput: move |event: FormEvent| {
                        if let Ok(bpm) = event.value().parse::<u32>() {
                            practice.set_bpm(bpm);
                        }
                    },
                }
            }
            div { class: "stage-actions",
                button {
                    class: "tap-button",
                    title: "Tap the tempo",
                    onmousedown: handle_tap,
                    "TAP"
                }
                button {
                    class: if running { "start-button running" } else { "start-button" },
                    title: if running { "Stop (Space)" } else { "Start (Space)" },
                    aria_label: if running { "Stop" } else { "Start" },
                    onclick: move |_| practice.toggle(),
                    if running {
                        Square { size: 26, fill: "currentColor" }
                    } else {
                        Play { size: 28, fill: "currentColor" }
                    }
                }
                div { class: "trainer-slot",
                    if options.trainer_on {
                        span { class: "trainer-pill",
                            if f64::from(bpm) >= options.trainer.up_to_bpm {
                                "Trainer · at {options.trainer.up_to_bpm} BPM"
                            } else {
                                "Trainer · {bpm} → {options.trainer.up_to_bpm} BPM"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn SidePanel() -> Element {
    let mut practice = use_practice();
    let options = practice.options();
    let trainer = options.trainer;

    rsx! {
        aside { class: "practice-panel",
            section { class: "panel-section",
                h2 { class: "control-label", "Time signature" }
                Segmented {
                    label: "Time signature",
                    options: Meter::ALL
                        .iter()
                        .map(|&meter| SegmentOption::new(meter.label(), options.meter == meter))
                        .collect::<Vec<_>>(),
                    onselect: move |i: usize| practice.set_meter(Meter::ALL[i]),
                }
            }
            section { class: "panel-section",
                h2 { class: "control-label", "Click on" }
                Segmented {
                    label: "Click on",
                    options: Subdivision::ALL
                        .iter()
                        .map(|&sub| SegmentOption::new(sub.label(), options.subdivision == sub))
                        .collect::<Vec<_>>(),
                    onselect: move |i: usize| practice.update(|o| o.subdivision = Subdivision::ALL[i]),
                }
            }
            section { class: "panel-section",
                h2 { class: "control-label", "Signals" }
                div { class: "panel-row",
                    span { "Sound" }
                    Switch {
                        checked: options.sound_on,
                        label: "Sound",
                        onchange: move |on| practice.update(|o| o.sound_on = on),
                    }
                }
                Segmented {
                    label: "Click sound",
                    options: SOUNDS
                        .iter()
                        .map(|&sound| SegmentOption::new(sound.label(), options.sound == sound))
                        .collect::<Vec<_>>(),
                    onselect: move |i: usize| practice.update(|o| o.sound = SOUNDS[i]),
                }
                label { class: "labelled-slider",
                    span { class: "slider-labels",
                        span { class: "control-label", "Volume" }
                        span { "{(options.volume * 100.0).round()}%" }
                    }
                    input {
                        r#type: "range",
                        style: "--fill: {options.volume * 100.0}%",
                        min: 0,
                        max: 100,
                        step: 1,
                        value: (options.volume * 100.0).round(),
                        oninput: move |event: FormEvent| {
                            if let Ok(percent) = event.value().parse::<f64>() {
                                practice.update(|o| o.volume = percent / 100.0);
                            }
                        },
                    }
                }
                div { class: "panel-row",
                    span { "Flash the screen on beat 1" }
                    Switch {
                        checked: options.flash,
                        label: "Flash the screen on beat 1",
                        onchange: move |on| practice.update(|o| o.flash = on),
                    }
                }
            }
            section { class: "panel-section",
                h2 { class: "control-label", "Practice" }
                div { class: "panel-row",
                    div { class: "panel-row-text",
                        span { "Speed trainer" }
                        span { class: "panel-hint",
                            "+"
                            NumberField {
                                label: "BPM added each time",
                                value: trainer.step_bpm as u32,
                                min: 1,
                                max: 50,
                                onchange: move |n: u32| practice.update(|o| o.trainer.step_bpm = f64::from(n)),
                            }
                            " BPM every "
                            NumberField {
                                label: "Bars between steps",
                                value: trainer.every_bars,
                                min: 1,
                                max: 64,
                                onchange: move |n: u32| practice.update(|o| o.trainer.every_bars = n),
                            }
                            " bars, up to "
                            NumberField {
                                label: "Fastest tempo",
                                value: trainer.up_to_bpm as u32,
                                min: MIN_BPM,
                                max: MAX_BPM,
                                onchange: move |n: u32| practice.update(|o| o.trainer.up_to_bpm = f64::from(n)),
                            }
                        }
                    }
                    Switch {
                        checked: options.trainer_on,
                        label: "Speed trainer",
                        onchange: move |on| practice.update(|o| o.trainer_on = on),
                    }
                }
                div { class: "panel-row",
                    div { class: "panel-row-text",
                        span { "Silent bar" }
                        span { class: "panel-hint",
                            "Mute every {ordinal(SILENT_BAR_EVERY)} bar to test your time"
                        }
                    }
                    Switch {
                        checked: options.silent_bars,
                        label: "Silent bar",
                        onchange: move |on| practice.update(|o| o.silent_bars = on),
                    }
                }
            }
        }
    }
}

/// A small number typed into a sentence, kept within `min` to `max`
#[component]
fn NumberField(
    label: String,
    value: u32,
    min: u32,
    max: u32,
    onchange: EventHandler<u32>,
) -> Element {
    rsx! {
        input {
            class: "inline-number",
            r#type: "number",
            aria_label: label,
            min,
            max,
            value,
            onchange: move |event: FormEvent| {
                if let Ok(n) = event.value().parse::<u32>() {
                    onchange.call(n.clamp(min, max));
                }
            },
        }
    }
}

fn beat_class(kind: BeatKind, beat: usize, click: Option<ClickPosition>) -> String {
    let mut class = String::from("beat-circle");
    class.push_str(match kind {
        BeatKind::Accent => " accent",
        BeatKind::Normal => " normal",
        BeatKind::Silent => " silent",
    });
    if let Some(click) = click
        && click.beat as usize == beat
    {
        class.push_str(" lit");
        if click.silent {
            class.push_str(" hollow");
        }
    }
    class
}

/// The tempo from taps on the beat, averaging the time between them
fn tapped_bpm(taps: &[Instant]) -> Option<u32> {
    let (first, last) = (taps.first()?, taps.last()?);
    let gaps = taps.len().checked_sub(1).filter(|&gaps| gaps > 0)?;
    let average = last.duration_since(*first).as_secs_f64() / gaps as f64;
    (average > 0.0).then(|| {
        (60.0 / average)
            .round()
            .clamp(f64::from(MIN_BPM), f64::from(MAX_BPM)) as u32
    })
}

fn ordinal(n: u32) -> String {
    let suffix = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn averages_taps() {
        let start = Instant::now();
        let taps: Vec<_> = [0, 500, 1000, 1500]
            .iter()
            .map(|&ms| start + Duration::from_millis(ms))
            .collect();
        assert_eq!(tapped_bpm(&taps), Some(120));
        assert_eq!(tapped_bpm(&taps[..1]), None);
    }
}

use std::time::Duration;

use dioxus::prelude::*;
use lucide_dioxus::{ChevronUp, Drum, RefreshCw, Volume1, Volume2, VolumeX};

use crate::{
    audio::{self, midi},
    settings::DrumsOutput,
    ui::{
        controls::{SegmentOption, Segmented},
        providers::playback_manager::use_playback_manager,
    },
};

/// How long the activity dot stays lit after a note is sent
const ACTIVITY_LIT: Duration = Duration::from_millis(120);

/// How often the activity dot and the port's status are checked
const POLL_INTERVAL: Duration = Duration::from_millis(40);

const OUTPUTS: [(DrumsOutput, &str); 2] = [
    (DrumsOutput::Synth, "Built-in"),
    (DrumsOutput::Midi, "MIDI"),
];

/// Mute, the volume or where MIDI notes are going, and a menu for picking
/// where the drums are played
#[component]
pub fn VolumeControl() -> Element {
    let mut playback_manager = use_playback_manager();
    let mut open = use_signal(|| false);
    let mut ports = use_signal(Vec::<String>::new);
    let mut activity = use_signal(|| false);
    let mut status = use_signal(midi::status);

    // The MIDI thread doesn't tell the UI anything, so look in on it
    use_future(move || async move {
        loop {
            tokio::time::sleep(POLL_INTERVAL).await;
            let lit = midi::since_last_sent().is_some_and(|since| since < ACTIVITY_LIT);
            if lit != *activity.peek() {
                activity.set(lit);
            }
            let current = midi::status();
            if current != *status.peek() {
                status.set(current);
            }
            let found = midi::ports();
            if found != *ports.peek() {
                ports.set(found);
            }
        }
    });

    let output = playback_manager.drums_output();
    let is_midi = output == DrumsOutput::Midi;
    let is_muted = playback_manager.is_muted();
    let port = playback_manager.midi_port();
    let offset_ms = playback_manager.midi_offset_ms();
    // A muted slider reads as empty, and dragging it unmutes
    let volume_percent = if is_muted {
        0.0
    } else {
        (playback_manager.volume() * 100.0).round()
    };

    let refresh_ports = midi::refresh_ports;

    let handle_toggle_menu = move |_: MouseEvent| {
        // Looking for ports starts the system's MIDI, so only once it's used
        if !open() && is_midi {
            refresh_ports();
        }
        open.toggle();
    };

    let handle_volume_input = move |event: FormEvent| {
        if let Ok(percent) = event.value().parse::<f64>() {
            playback_manager.set_volume(percent / 100.0);
        }
    };

    let handle_offset_input = move |event: FormEvent| {
        if let Ok(ms) = event.value().parse::<u32>() {
            playback_manager.set_midi_offset_ms(ms);
        }
    };

    // The picked port stays in the list while it's missing, so it's clear
    // what's picked
    let mut listed_ports = ports();
    let port_missing = !listed_ports.contains(&port);
    if port_missing {
        listed_ports.push(port.clone());
    }

    let (target_class, target_text) = if is_muted {
        ("midi-target-value muted", "Muted".to_string())
    } else {
        match status() {
            midi::Status::Missing => ("midi-target-value problem", format!("{port} missing")),
            midi::Status::Failed => ("midi-target-value problem", "Unavailable".to_string()),
            _ => ("midi-target-value", port.clone()),
        }
    };
    let dot_class = match (status(), is_muted, activity()) {
        (midi::Status::Missing | midi::Status::Failed, false, _) => "activity-dot problem",
        (_, false, true) => "activity-dot lit",
        _ => "activity-dot",
    };

    rsx! {
        div { class: "volume-control",
            button {
                class: "mute-button",
                title: match (is_muted, is_midi) {
                    (true, _) => "Unmute (M)",
                    (false, true) => "Stop sending notes (M)",
                    (false, false) => "Mute (M)",
                },
                onclick: move |_| playback_manager.toggle_mute(),
                if is_muted || (!is_midi && volume_percent == 0.0) {
                    VolumeX { size: 18 }
                } else if !is_midi && volume_percent < 50.0 {
                    Volume1 { size: 18 }
                } else {
                    Volume2 { size: 18 }
                }
            }
            if is_midi {
                div {
                    class: "midi-target",
                    title: "Where the drums' MIDI notes are going",
                    span { class: "slider-labels",
                        span { class: "control-label", "MIDI out" }
                    }
                    span { class: target_class,
                        span { class: dot_class }
                        span { class: "midi-target-name", "{target_text}" }
                    }
                }
            } else {
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
            button {
                class: if open() { "output-menu open" } else { "output-menu" },
                title: "Drums output",
                aria_label: "Drums output",
                aria_haspopup: "dialog",
                aria_expanded: open(),
                onclick: handle_toggle_menu,
                ChevronUp { size: 14, class: "popover-chevron" }
            }
            if open() {
                // Clicking anywhere outside the popover closes it
                div {
                    class: "popover-backdrop",
                    onclick: move |_| open.set(false),
                }
                div {
                    class: "output-popover",
                    role: "dialog",
                    aria_label: "Drums output",
                    onkeydown: move |event: KeyboardEvent| {
                        if event.key().to_string() == "Escape" {
                            open.set(false);
                        }
                    },
                    div { class: "popover-heading",
                        span { class: "control-label", "Drums output" }
                        span { class: "popover-subtitle", "Where the score's drums are played" }
                    }
                    Segmented {
                        label: "Drums output",
                        options: OUTPUTS
                            .iter()
                            .map(|&(option, label)| SegmentOption::new(label, option == output))
                            .collect::<Vec<_>>(),
                        onselect: move |i: usize| {
                            let output = OUTPUTS[i].0;
                            if output == DrumsOutput::Midi {
                                refresh_ports();
                            }
                            playback_manager.set_drums_output(output);
                        },
                    }
                    if is_midi {
                        div { class: "popover-field",
                            div { class: "field-header",
                                span { class: "control-label", "Port" }
                                button {
                                    class: "small-button",
                                    title: "Look for ports again",
                                    onclick: move |_| {
                                        refresh_ports();
                                        playback_manager.reconnect_midi();
                                    },
                                    RefreshCw { size: 12 }
                                    "Refresh"
                                }
                            }
                            div {
                                class: "port-list",
                                role: "radiogroup",
                                aria_label: "Port",
                                for name in listed_ports {
                                    button {
                                        key: "{name}",
                                        class: if name == port { "port selected" } else { "port" },
                                        role: "radio",
                                        aria_checked: name == port,
                                        onclick: {
                                            let name = name.clone();
                                            move |_| playback_manager.set_midi_port(name.clone())
                                        },
                                        span { class: "radio" }
                                        span { class: "port-name", "{name}" }
                                        if name == midi::VIRTUAL_PORT {
                                            span { class: "tag", "Virtual" }
                                        } else if name == port && status() == midi::Status::Missing {
                                            span { class: "tag problem", "Missing" }
                                        }
                                    }
                                }
                            }
                        }
                        label {
                            class: "labelled-slider",
                            title: "Sends notes ahead of the playhead, to make up for the delay of whatever plays them",
                            span { class: "slider-labels",
                                span { class: "control-label", "Send early" }
                                span { "{offset_ms} ms" }
                            }
                            input {
                                r#type: "range",
                                style: "--fill: {f64::from(offset_ms) / f64::from(audio::MAX_SEND_EARLY_MS) * 100.0}%",
                                min: 0,
                                max: audio::MAX_SEND_EARLY_MS,
                                step: 5,
                                value: offset_ms,
                                oninput: handle_offset_input,
                            }
                        }
                        div { class: "popover-divider" }
                        div { class: "status-row",
                            match status() {
                                midi::Status::Connected => rsx! {
                                    span { class: "status",
                                        span { class: "status-dot ok" }
                                        "Sending on channel 10"
                                    }
                                },
                                midi::Status::Missing => rsx! {
                                    span { class: "status",
                                        span { class: "status-dot problem" }
                                        "{port} isn't connected"
                                    }
                                },
                                midi::Status::Failed => rsx! {
                                    span { class: "status",
                                        span { class: "status-dot problem" }
                                        "MIDI isn't available"
                                    }
                                },
                                midi::Status::Off => rsx! {
                                    span { class: "status",
                                        span { class: "status-dot" }
                                        "Connecting…"
                                    }
                                },
                            }
                            button {
                                class: "test-button",
                                title: "Send a snare hit now",
                                disabled: status() != midi::Status::Connected,
                                onclick: move |_| midi::test_hit(),
                                Drum { size: 14 }
                                "Test hit"
                            }
                        }
                        p { class: "hint",
                            if port == midi::VIRTUAL_PORT {
                                "Turn on the "
                                b { "{midi::VIRTUAL_PORT}" }
                                " MIDI input in your DAW or drum plugin. "
                            }
                            "Set the volume there; mute here stops sending notes."
                        }
                    } else {
                        p { class: "hint",
                            "Synthesized drums played by DrumBeat. Volume and mute apply."
                        }
                    }
                }
            }
        }
    }
}

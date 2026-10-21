use std::time::Duration;

use dioxus::prelude::*;
use lucide_dioxus::{Bluetooth, ChevronDown, Drum};

use crate::{
    audio::midi_in::{ReceivedHit, Status},
    data::{drum_piece::DrumPiece, drum_voice::DrumVoice},
    platform,
    ui::providers::midi_input::use_midi_input,
};

/// How often the inputs are looked for while the menu is open, so a kit
/// plugged in or paired meanwhile shows up
const LOOK_INTERVAL: Duration = Duration::from_secs(1);

/// How many earlier hits trail the last one
const TRAIL: usize = 8;

/// The MIDI input DrumBeat listens to: a chip naming it that flashes with
/// every hit, or on a phone a drum button with a dot, and a menu for picking
/// it with a readout of what's arriving
#[component]
pub fn MidiInputControl() -> Element {
    let mut input = use_midi_input();
    let mut open = use_signal(|| false);

    use_future(move || async move {
        loop {
            tokio::time::sleep(LOOK_INTERVAL).await;
            if open() {
                input.refresh();
            }
        }
    });

    let port = input.port();
    let status = input.status();
    let lit = input.is_lit();
    let problem = port.is_some() && matches!(status, Status::Missing | Status::Failed);

    let handle_toggle = move |_: MouseEvent| {
        // Looking for inputs starts the system's MIDI, so only once it's used
        if !open() {
            input.refresh();
        }
        open.toggle();
    };

    let chip_class = format!(
        "input-chip regular-only{}{}",
        match (&port, problem) {
            (None, _) => " unset",
            (Some(_), true) => " problem",
            _ => "",
        },
        if open() { " open" } else { "" },
    );
    let chip_name = match (&port, status) {
        (None, _) => "Choose kit".to_string(),
        (Some(port), Status::Missing) => format!("{port} missing"),
        (Some(_), Status::Failed) => "MIDI unavailable".to_string(),
        (Some(port), _) => port.clone(),
    };
    let dot_class = match (&port, problem, lit) {
        (None, _, _) => None,
        (Some(_), true, _) => Some("activity-dot problem"),
        (Some(_), false, true) => Some("activity-dot lit"),
        _ => Some("activity-dot"),
    };
    let badge_class = match (&port, problem, lit) {
        (None, _, _) => None,
        (Some(_), true, _) => Some("input-badge problem"),
        (Some(_), false, true) => Some("input-badge lit"),
        (Some(_), false, false) if status == Status::Connected => Some("input-badge ok"),
        _ => Some("input-badge"),
    };

    // The picked input stays in the list while it's missing, so it's clear
    // what's picked
    let mut listed = input.ports();
    if let Some(port) = &port
        && !listed.contains(port)
    {
        listed.push(port.clone());
    }

    rsx! {
        div { class: "midi-input-control",
            button {
                class: "{chip_class}",
                title: "MIDI input: the drum kit you play",
                aria_label: "MIDI input",
                aria_haspopup: "dialog",
                aria_expanded: open(),
                onclick: handle_toggle,
                Drum { size: 16 }
                if let Some(class) = dot_class {
                    span { class }
                }
                span { class: "input-chip-name", "{chip_name}" }
                ChevronDown { size: 14, class: "popover-chevron" }
            }
            button {
                class: if open() { "more-button input-button compact-only open" } else { "more-button input-button compact-only" },
                aria_label: "MIDI input",
                aria_haspopup: "dialog",
                aria_expanded: open(),
                onclick: handle_toggle,
                Drum { size: 20 }
                if let Some(class) = badge_class {
                    span { class }
                }
            }
            if open() {
                // Clicking anywhere outside the menu closes it
                div {
                    class: "toolbar-popover-backdrop",
                    onclick: move |_| open.set(false),
                }
                div {
                    class: "input-popover",
                    role: "dialog",
                    aria_label: "MIDI input",
                    onkeydown: move |event: KeyboardEvent| {
                        if event.key().to_string() == "Escape" {
                            open.set(false);
                        }
                    },
                    div { class: "sheet-header compact-only",
                        div { class: "popover-heading",
                            h2 { "MIDI input" }
                            span { class: "popover-subtitle", "The drum kit or pads you play" }
                        }
                        button {
                            class: "sheet-done",
                            onclick: move |_| open.set(false),
                            "Done"
                        }
                    }
                    div { class: "popover-heading regular-only",
                        span { class: "control-label", "MIDI input" }
                        span { class: "popover-subtitle", "The drum kit or pads you play" }
                    }
                    div { class: "popover-field",
                        span { class: "control-label", "Device" }
                        div {
                            class: "port-list",
                            role: "radiogroup",
                            aria_label: "Device",
                            if listed.is_empty() {
                                p { class: "empty-ports",
                                    "No MIDI inputs found. Plug your kit in."
                                }
                            } else {
                                button {
                                    class: if port.is_none() { "port selected" } else { "port" },
                                    role: "radio",
                                    aria_checked: port.is_none(),
                                    onclick: move |_| input.set_port(None),
                                    span { class: "radio" }
                                    span { class: "port-name", "None" }
                                }
                                for name in listed {
                                    PortRow {
                                        key: "{name}",
                                        selected: port.as_ref() == Some(&name),
                                        name,
                                    }
                                }
                            }
                            if platform::CAN_PAIR_BLUETOOTH_MIDI {
                                button {
                                    class: "port",
                                    onclick: move |_| {
                                        open.set(false);
                                        platform::pair_bluetooth_midi();
                                    },
                                    span { class: "port-icon", Bluetooth { size: 16 } }
                                    span { class: "port-name", "Bluetooth MIDI…" }
                                    span { class: "tag", "Pair" }
                                }
                            }
                        }
                    }
                    if port.is_some() && status == Status::Connected {
                        div { class: "popover-divider" }
                        HitMonitor { hits: input.hits() }
                    }
                    div { class: "popover-divider" }
                    div { class: "status-row",
                        span { class: "status",
                            match (&port, status) {
                                (None, _) => rsx! {
                                    span { class: "status-dot" }
                                    "Not listening"
                                },
                                (Some(port), Status::Connected) => rsx! {
                                    span { class: "status-dot ok" }
                                    "Listening to {port}"
                                },
                                (Some(port), Status::Missing) => rsx! {
                                    span { class: "status-dot problem" }
                                    "{port} isn't connected"
                                },
                                (Some(_), Status::Failed) => rsx! {
                                    span { class: "status-dot problem" }
                                    "MIDI isn't available"
                                },
                                (Some(_), Status::Off) => rsx! {
                                    span { class: "status-dot" }
                                    "Connecting…"
                                },
                            }
                        }
                    }
                    p { class: "hint",
                        match (&port, status) {
                            (None, _) => "Pick the kit or pad controller you play on.",
                            (Some(_), Status::Missing) => "Plug it back in to reconnect.",
                            _ => "Notes on any channel are taken.",
                        }
                    }
                }
            }
        }
    }
}

/// One input in the menu's list
#[component]
fn PortRow(name: String, selected: bool) -> Element {
    let mut input = use_midi_input();
    let status = input.status();
    let lit = input.is_lit();

    rsx! {
        button {
            class: if selected { "port selected" } else { "port" },
            role: "radio",
            aria_checked: selected,
            onclick: {
                let name = name.clone();
                move |_| input.set_port(Some(name.clone()))
            },
            span { class: "radio" }
            span { class: "port-name", "{name}" }
            if selected {
                match status {
                    Status::Missing => rsx! {
                        span { class: "tag problem", "Missing" }
                    },
                    Status::Connected => rsx! {
                        span { class: if lit { "activity-dot port-dot lit" } else { "activity-dot port-dot" } }
                    },
                    _ => rsx! {},
                }
            }
        }
    }
}

/// The last hit, named, with its note and velocity, and the hits before it
#[component]
fn HitMonitor(hits: Vec<ReceivedHit>) -> Element {
    let last = hits.first().copied();
    let velocity_percent = last.map_or(0, |hit| u32::from(hit.velocity) * 100 / 127);

    rsx! {
        div { class: "hit-monitor",
            span { class: "control-label", "Last hit" }
            div { class: "last-hit",
                if let Some(hit) = last {
                    span { class: "last-hit-name", "{hit_name(hit.note)}" }
                    span { class: "last-hit-meta",
                        "Note {hit.note} · vel {hit.velocity} · ch {hit.channel}"
                    }
                } else {
                    span { class: "last-hit-name waiting", "Hit a pad to check it's coming through" }
                }
                div { class: "velocity", "aria-hidden": "true",
                    span { style: "width: {velocity_percent}%" }
                }
            }
            div { class: "hit-trail", aria_label: "Earlier hits",
                for (i , hit) in hits.iter().take(TRAIL).enumerate() {
                    span {
                        key: "{i}",
                        title: "{hit_name(hit.note)}, note {hit.note}",
                        "{short_name(hit.note)} {hit.velocity}"
                    }
                }
            }
        }
    }
}

/// The drum a note plays in General MIDI, as Guitar Pro maps it
fn hit_name(note: u8) -> String {
    DrumVoice::from_gp_note(note)
        .map_or_else(|| format!("Note {note}"), |voice| voice.name().to_string())
}

/// A few letters for the drum a note plays, for the trail of hits
fn short_name(note: u8) -> String {
    let Some(voice) = DrumVoice::from_gp_note(note) else {
        return note.to_string();
    };
    match voice {
        DrumVoice::HihatOpen => "HH open",
        DrumVoice::SideStick => "Stick",
        DrumVoice::RimShot => "Rim",
        DrumVoice::RideBell => "Bell",
        _ => match voice.piece() {
            DrumPiece::Kick => "Kick",
            DrumPiece::Kick2 => "Kick 2",
            DrumPiece::Snare => "Snare",
            DrumPiece::Hihat => "HH",
            DrumPiece::HihatPedal => "Pedal",
            DrumPiece::HighTom => "Tom 1",
            DrumPiece::MidTom => "Tom 2",
            DrumPiece::LowTom => "Tom 3",
            DrumPiece::VeryLowTom => "Tom 4",
            DrumPiece::FloorTom => "Floor",
            DrumPiece::LowFloorTom => "Floor 2",
            DrumPiece::Crash => "Crash",
            DrumPiece::Ride => "Ride",
            DrumPiece::China => "China",
            DrumPiece::Splash => "Splash",
            DrumPiece::Cowbell => "Cowbell",
        },
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_hits() {
        assert_eq!(hit_name(38), "Snare");
        assert_eq!(hit_name(20), "Note 20");
        assert_eq!(short_name(42), "HH");
        assert_eq!(short_name(46), "HH open");
        assert_eq!(short_name(20), "20");
    }
}

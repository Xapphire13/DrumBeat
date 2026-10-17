#![warn(clippy::pedantic)]

use std::rc::Rc;

use dioxus::prelude::*;
use midir::MidiInput;

use crate::ui::{
    providers::{
        playback_manager::{use_playback_manager, use_playback_manager_provider},
        score_manager::{load_score, use_score_manager, use_score_manager_provider},
    },
    score::Score,
};

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");

#[component]
pub fn App() -> Element {
    use_playback_manager_provider();
    use_score_manager_provider();

    let mut midi_output = use_signal_sync(String::new);
    let _midi_connection = use_hook(|| {
        let midi_in = MidiInput::new("DrumBeat").unwrap();

        midi_output.set(format!("Number of ports: {}", midi_in.ports().len()));

        if midi_in.ports().is_empty() {
            None
        } else {
            let port = &midi_in.ports()[0];
            let port_name = midi_in.port_name(port).unwrap();

            let connection = midi_in.connect(
                port,
                port_name.as_str(),
                move |_, message, ()| {
                    midi_output.set(format!("{message:?}"));
                },
                (),
            );

            Some(Rc::new(connection))
        }
    });

    rsx! {
        {dioxus_feather_icons::sprite!()}
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }

        Toolbar {}
        div { "MIDI: {midi_output}" }

        Score {}
    }
}

#[component]
fn Toolbar() -> Element {
    let mut score = use_score_manager();
    let mut playback_manager = use_playback_manager();
    let mut error = use_signal(|| None::<String>);

    let handle_open = move |_: MouseEvent| {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Guitar Pro 8", &["gp"])
            .pick_file()
        else {
            return;
        };

        match load_score(&path) {
            Ok(new_score) => {
                playback_manager.reset();
                score.set(new_score);
                error.set(None);
            }
            Err(e) => error.set(Some(format!("Couldn't open {}: {e}", path.display()))),
        }
    };

    rsx! {
        div { class: "toolbar",
            button { onclick: handle_open, "Open…" }
            span { class: "score-title", "{score().title}" }
            if let Some(error) = error() {
                span { class: "error", "{error}" }
            }
        }
    }
}

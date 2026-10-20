use std::time::Duration;

use dioxus::prelude::*;

use crate::{
    audio::midi_in::{self, ReceivedHit, Status},
    settings::{self, MidiInputSettings},
};

/// How long the activity dot stays lit after a hit
const ACTIVITY_LIT: Duration = Duration::from_millis(120);

/// How often the input is looked in on for new hits and its status
const POLL_INTERVAL: Duration = Duration::from_millis(40);

/// The MIDI input DrumBeat listens to, usually the drum kit, and what's
/// arriving from it
#[derive(Clone, Copy)]
pub struct MidiInput {
    /// The input picked, kept between launches. Empty for none
    port: Signal<String>,
    status: Signal<Status>,
    /// The system's inputs, as last found
    ports: Signal<Vec<String>>,
    /// The latest hits, newest first
    hits: Signal<Vec<ReceivedHit>>,
    /// Set for a moment after each hit
    lit: Signal<bool>,
}

impl MidiInput {
    /// The input picked, or `None` while not listening
    pub fn port(&self) -> Option<String> {
        Some((self.port)()).filter(|port| !port.is_empty())
    }

    pub fn status(&self) -> Status {
        (self.status)()
    }

    pub fn ports(&self) -> Vec<String> {
        (self.ports)()
    }

    pub fn hits(&self) -> Vec<ReceivedHit> {
        (self.hits)()
    }

    pub fn is_lit(&self) -> bool {
        (self.lit)()
    }

    /// Listen to `port`, or stop listening with `None`
    pub fn set_port(&mut self, port: Option<String>) {
        let port = port.unwrap_or_default();
        settings::save_midi_input(&MidiInputSettings { port: port.clone() });
        self.port.set(port);
        self.hits.set(vec![]);
        midi_in::connect(self.port());
    }

    /// Look for inputs again, and try the picked one again if it isn't
    /// connected
    pub fn refresh(&self) {
        midi_in::refresh_ports();
        if self.status() != Status::Connected {
            midi_in::connect(self.port());
        }
    }
}

pub fn use_midi_input_provider() -> MidiInput {
    let input = use_context_provider(|| {
        let port = settings::load().midi_input.port;
        // The system's MIDI is only started once an input is picked
        if !port.is_empty() {
            midi_in::connect(Some(port.clone()));
        }
        MidiInput {
            port: Signal::new(port),
            status: Signal::new(midi_in::status()),
            ports: Signal::new(vec![]),
            hits: Signal::new(vec![]),
            lit: Signal::new(false),
        }
    });

    // The input thread doesn't tell the UI anything, so look in on it
    let MidiInput {
        mut status,
        mut ports,
        mut hits,
        mut lit,
        ..
    } = input;
    use_future(move || async move {
        let mut seen = midi_in::hits_received();
        loop {
            tokio::time::sleep(POLL_INTERVAL).await;
            let current = midi_in::status();
            if current != *status.peek() {
                status.set(current);
            }
            let found = midi_in::ports();
            if found != *ports.peek() {
                ports.set(found);
            }
            let received = midi_in::hits_received();
            if received != seen {
                seen = received;
                hits.set(midi_in::recent_hits());
            }
            let is_lit = hits
                .peek()
                .first()
                .is_some_and(|hit| hit.at.elapsed() < ACTIVITY_LIT);
            if is_lit != *lit.peek() {
                lit.set(is_lit);
            }
        }
    });

    input
}

pub fn use_midi_input() -> MidiInput {
    use_context::<MidiInput>()
}

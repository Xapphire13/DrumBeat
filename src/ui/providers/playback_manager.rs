use dioxus::prelude::*;

#[derive(Clone)]
pub struct PlaybackState {
    pub playhead_timestamp: u32,
}

pub fn use_playback_manager_provider() {
    use_context_provider(|| {
        Signal::new(PlaybackState {
            playhead_timestamp: 0,
        })
    });
}

#[derive(Clone, Copy)]
pub struct PlaybackManager {
    state: Signal<PlaybackState>,
}

impl PlaybackManager {
    pub fn update_playhead(&mut self, new_playhead: u32) {
        let current_state = (self.state)();

        self.state.set(PlaybackState {
            playhead_timestamp: new_playhead,
            ..current_state
        });
    }

    pub fn playhead_timestamp(&self) -> u32 {
        (self.state)().playhead_timestamp
    }
}

pub fn use_playback_manager() -> PlaybackManager {
    let playback_state = use_context::<Signal<PlaybackState>>();

    PlaybackManager {
        state: playback_state,
    }
}

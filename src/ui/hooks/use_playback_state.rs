use dioxus::prelude::*;

#[derive(Clone)]
pub struct PlaybackState {
    pub playhead_timestamp: u32,
}

pub fn use_playback_state_provider() -> Signal<PlaybackState> {
    use_context_provider(|| {
        Signal::new(PlaybackState {
            playhead_timestamp: 0,
        })
    })
}

pub fn use_playback_state() -> Signal<PlaybackState> {
    use_context::<Signal<PlaybackState>>()
}

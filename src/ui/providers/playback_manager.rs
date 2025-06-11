use dioxus::prelude::*;

use crate::data::score::Score;

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
    pub fn new(state: Signal<PlaybackState>) -> Self {
        Self { state }
    }

    pub fn update_playhead(&mut self, new_playhead: u32, score: &Score) {
        let timestamps = score.get_note_timestamps();

        if timestamps
            .last()
            .is_none_or(|&last_timestamp| new_playhead > last_timestamp)
        {
            return;
        }

        self.state.set(PlaybackState {
            playhead_timestamp: new_playhead,
        });
    }

    pub fn playhead_timestamp(&self) -> u32 {
        (self.state)().playhead_timestamp
    }

    pub fn calculate_measure_index(&self, score: &Score) -> usize {
        score
            .measures
            .iter()
            .enumerate()
            .find(|(_, measure)| measure.timestamp > self.playhead_timestamp())
            .map_or_else(
                || score.measures.len().saturating_sub(1),
                |(i, ..)| i.saturating_sub(1),
            )
    }

    pub fn skip_to_measure(&mut self, measure_index: usize, score: &Score) {
        if let Some(measure) = score.measures.get(measure_index) {
            self.update_playhead(measure.timestamp, score);
        }
    }

    pub fn skip_to_next_note(&mut self, score: &Score) {
        let timestamps = score.get_note_timestamps();
        if let Some(current_index) = timestamps
            .iter()
            .position(|&ts| ts == self.playhead_timestamp())
        {
            if let Some(&next_timestamp) = timestamps.get(current_index + 1) {
                self.update_playhead(next_timestamp, score);
            }
        }
    }

    pub fn skip_to_previous_note(&mut self, score: &Score) {
        let timestamps = score.get_note_timestamps();
        if let Some(current_index) = timestamps
            .iter()
            .position(|&ts| ts == self.playhead_timestamp())
        {
            if let Some(&prev_timestamp) = timestamps.get(current_index.saturating_sub(1)) {
                self.update_playhead(prev_timestamp, score);
            }
        }
    }
}

pub fn use_playback_manager() -> PlaybackManager {
    let playback_state = use_context::<Signal<PlaybackState>>();

    PlaybackManager::new(playback_state)
}

use std::time::{Duration, Instant};

use dioxus::{core::Task, prelude::*};

use crate::data::score::Score;

/// How often the playhead moves while playing
const FRAME_INTERVAL: Duration = Duration::from_millis(16);

#[derive(Clone)]
pub struct PlaybackState {
    pub playhead_timestamp: u32,
    /// Set while the score is playing
    clock: Option<PlaybackClock>,
}

/// Ties the playhead to real time: `start_seconds` into the score was
/// reached at `started_at`
#[derive(Clone, Copy)]
struct PlaybackClock {
    started_at: Instant,
    start_seconds: f64,
    task: Task,
}

pub fn use_playback_manager_provider() {
    use_context_provider(|| {
        Signal::new(PlaybackState {
            playhead_timestamp: 0,
            clock: None,
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

        let mut state = self.state.write();
        state.playhead_timestamp = new_playhead;
        // Keep playing from the new position
        if let Some(clock) = &mut state.clock {
            clock.started_at = Instant::now();
            clock.start_seconds = score.tempo_map.ticks_to_seconds(new_playhead);
        }
    }

    pub fn playhead_timestamp(&self) -> u32 {
        (self.state)().playhead_timestamp
    }

    pub fn is_playing(&self) -> bool {
        (self.state)().clock.is_some()
    }

    /// Start moving the playhead in real time, following the score's tempo
    pub fn play(&mut self, score: &Score) {
        if self.state.peek().clock.is_some() || score.measures.is_empty() {
            return;
        }

        let mut state = self.state;
        let tempo_map = score.tempo_map.clone();
        let end_timestamp = score.end_timestamp();
        let start_timestamp = if state.peek().playhead_timestamp >= end_timestamp {
            0
        } else {
            state.peek().playhead_timestamp
        };

        let task = spawn(async move {
            loop {
                tokio::time::sleep(FRAME_INTERVAL).await;

                let Some(clock) = state.peek().clock else {
                    break;
                };
                let seconds = clock.start_seconds + clock.started_at.elapsed().as_secs_f64();
                let timestamp = tempo_map.seconds_to_ticks(seconds);

                let mut state = state.write();
                if timestamp >= f64::from(end_timestamp) {
                    // Finished: rewind so the next play starts from the top
                    state.clock = None;
                    state.playhead_timestamp = 0;
                    break;
                }
                state.playhead_timestamp = timestamp as u32;
            }
        });

        let mut state = state.write();
        state.playhead_timestamp = start_timestamp;
        state.clock = Some(PlaybackClock {
            started_at: Instant::now(),
            start_seconds: score.tempo_map.ticks_to_seconds(start_timestamp),
            task,
        });
    }

    /// Stop moving the playhead, leaving it where it is
    pub fn pause(&mut self) {
        if let Some(clock) = self.state.write().clock.take() {
            clock.task.cancel();
        }
    }

    /// Stop playing and move the playhead back to the start, e.g. when a new
    /// score is opened
    pub fn reset(&mut self) {
        self.pause();
        self.state.write().playhead_timestamp = 0;
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
        let playhead = self.playhead_timestamp();
        if let Some(next_timestamp) = score
            .get_note_timestamps()
            .into_iter()
            .find(|&ts| ts > playhead)
        {
            self.update_playhead(next_timestamp, score);
        }
    }

    pub fn skip_to_previous_note(&mut self, score: &Score) {
        let playhead = self.playhead_timestamp();
        if let Some(prev_timestamp) = score
            .get_note_timestamps()
            .into_iter()
            .rev()
            .find(|&ts| ts < playhead)
        {
            self.update_playhead(prev_timestamp, score);
        }
    }
}

pub fn use_playback_manager() -> PlaybackManager {
    let playback_state = use_context::<Signal<PlaybackState>>();

    PlaybackManager::new(playback_state)
}

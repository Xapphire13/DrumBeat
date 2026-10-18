use std::time::{Duration, Instant};

use dioxus::{core::Task, prelude::*};

use crate::data::score::Score;

/// How often the playhead moves while playing
const FRAME_INTERVAL: Duration = Duration::from_millis(16);

/// The slowest and fastest playback speeds, as a fraction of the score's tempo
const MIN_SPEED: f64 = 0.1;
const MAX_SPEED: f64 = 4.0;

#[derive(Clone)]
pub struct PlaybackState {
    /// Where the playhead is drawn, on the score's written timeline
    pub playhead_timestamp: u32,
    /// Where the playhead is on the performance timeline, which tells repeated
    /// passes of the same measure apart
    performance_timestamp: u32,
    /// Multiplies the score's tempo, e.g. 0.5 plays at half speed
    speed: f64,
    /// Set while the score is playing
    clock: Option<PlaybackClock>,
}

/// Ties the playhead to real time: `start_seconds` into the performance (at
/// the score's own tempo) was reached at `started_at`
#[derive(Clone, Copy)]
struct PlaybackClock {
    started_at: Instant,
    start_seconds: f64,
    task: Task,
}

impl PlaybackClock {
    /// Seconds into the performance at the score's own tempo
    fn seconds(&self, speed: f64) -> f64 {
        self.start_seconds + self.started_at.elapsed().as_secs_f64() * speed
    }
}

pub fn use_playback_manager_provider() {
    use_context_provider(|| {
        Signal::new(PlaybackState {
            playhead_timestamp: 0,
            performance_timestamp: 0,
            speed: 1.0,
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

    /// Move the playhead to a written timestamp. When it is in a repeated
    /// passage, the pass nearest the playhead is used
    pub fn update_playhead(&mut self, new_playhead: u32, score: &Score) {
        let performance = &score.performance;
        let current = self.state.peek().performance_timestamp;
        let Some(timestamp) = performance.to_performance(new_playhead, current) else {
            return;
        };

        if timestamp >= performance.end_timestamp() {
            return;
        }

        self.seek(timestamp, score);
    }

    /// Move the playhead to a performance timestamp
    fn seek(&mut self, timestamp: u32, score: &Score) {
        let mut state = self.state.write();
        state.performance_timestamp = timestamp;
        state.playhead_timestamp = score.performance.to_written(timestamp);
        // Keep playing from the new position
        if let Some(clock) = &mut state.clock {
            clock.started_at = Instant::now();
            clock.start_seconds = score.performance.tempo_map().ticks_to_seconds(timestamp);
        }
    }

    pub fn playhead_timestamp(&self) -> u32 {
        (self.state)().playhead_timestamp
    }

    pub fn performance_timestamp(&self) -> u32 {
        (self.state)().performance_timestamp
    }

    pub fn is_playing(&self) -> bool {
        (self.state)().clock.is_some()
    }

    pub fn speed(&self) -> f64 {
        (self.state)().speed
    }

    /// Change how fast the score plays, relative to its own tempo
    pub fn set_speed(&mut self, speed: f64) {
        let speed = speed.clamp(MIN_SPEED, MAX_SPEED);
        let mut state = self.state.write();
        let old_speed = state.speed;
        // Restart the clock from where it is now, so the playhead doesn't jump
        if let Some(clock) = &mut state.clock {
            clock.start_seconds = clock.seconds(old_speed);
            clock.started_at = Instant::now();
        }
        state.speed = speed;
    }

    /// Start moving the playhead in real time, following the score's tempo
    /// and repeats
    pub fn play(&mut self, score: &Score) {
        if self.state.peek().clock.is_some() || score.measures.is_empty() {
            return;
        }

        let mut state = self.state;
        let performance = score.performance.clone();
        let end_timestamp = performance.end_timestamp();
        let start_timestamp = if state.peek().performance_timestamp >= end_timestamp {
            0
        } else {
            state.peek().performance_timestamp
        };
        let start_seconds = performance.tempo_map().ticks_to_seconds(start_timestamp);

        let task = spawn(async move {
            loop {
                tokio::time::sleep(FRAME_INTERVAL).await;

                let seconds = {
                    let state = state.peek();
                    let Some(clock) = state.clock else {
                        break;
                    };
                    clock.seconds(state.speed)
                };
                let timestamp = performance.tempo_map().seconds_to_ticks(seconds);

                let mut state = state.write();
                if timestamp >= f64::from(end_timestamp) {
                    // Finished: rewind so the next play starts from the top
                    state.clock = None;
                    state.performance_timestamp = 0;
                    state.playhead_timestamp = 0;
                    break;
                }
                state.performance_timestamp = timestamp as u32;
                state.playhead_timestamp = performance.to_written(timestamp as u32);
            }
        });

        let mut state = state.write();
        state.performance_timestamp = start_timestamp;
        state.playhead_timestamp = score.performance.to_written(start_timestamp);
        state.clock = Some(PlaybackClock {
            started_at: Instant::now(),
            start_seconds,
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
        let mut state = self.state.write();
        state.playhead_timestamp = 0;
        state.performance_timestamp = 0;
    }

    /// Move the playhead by whole measures, in the order they are played
    pub fn skip_measures(&mut self, delta: isize, score: &Score) {
        let performance = &score.performance;
        let current = performance.played_measure_index(self.state.peek().performance_timestamp);
        let target = current
            .saturating_add_signed(delta)
            .min(performance.measures().len().saturating_sub(1));
        if let Some(played) = performance.measures().get(target) {
            self.seek(played.timestamp, score);
        }
    }

    pub fn skip_to_next_note(&mut self, score: &Score) {
        let playhead = self.state.peek().performance_timestamp;
        if let Some(next_timestamp) = score
            .performance
            .note_timestamps(&score.measures)
            .into_iter()
            .find(|&ts| ts > playhead)
        {
            self.seek(next_timestamp, score);
        }
    }

    pub fn skip_to_previous_note(&mut self, score: &Score) {
        let playhead = self.state.peek().performance_timestamp;
        if let Some(prev_timestamp) = score
            .performance
            .note_timestamps(&score.measures)
            .into_iter()
            .rev()
            .find(|&ts| ts < playhead)
        {
            self.seek(prev_timestamp, score);
        }
    }
}

pub fn use_playback_manager() -> PlaybackManager {
    let playback_state = use_context::<Signal<PlaybackState>>();

    PlaybackManager::new(playback_state)
}

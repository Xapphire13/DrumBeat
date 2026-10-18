use std::time::{Duration, Instant};

use dioxus::{core::Task, prelude::*};

use crate::{
    audio::{self, wrap_into_loop},
    data::{loop_range::LoopRange, score::Score},
};

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
    /// From 0 to 1
    volume: f64,
    muted: bool,
    /// The section being practised, which plays over and over. The playhead
    /// is kept inside it
    loop_range: Option<LoopRange>,
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
            volume: 1.0,
            muted: false,
            loop_range: None,
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

    /// Move the playhead to a performance timestamp. Moving it out of the
    /// loop stops looping
    pub fn seek(&mut self, timestamp: u32, score: &Score) {
        let mut state = self.state.write();
        if state
            .loop_range
            .is_some_and(|range| !range.contains(timestamp))
        {
            state.loop_range = None;
            audio::set_loop(None);
        }
        state.performance_timestamp = timestamp;
        state.playhead_timestamp = score.performance.to_written(timestamp);
        // Keep playing from the new position
        if let Some(clock) = &mut state.clock {
            clock.started_at = Instant::now();
            clock.start_seconds = score.performance.tempo_map().ticks_to_seconds(timestamp);
            audio::seek(clock.start_seconds);
        }
    }

    pub fn loop_range(&self) -> Option<LoopRange> {
        (self.state)().loop_range
    }

    /// Start looping `range`, moving the playhead to its start if it's
    /// outside it
    pub fn set_loop(&mut self, range: LoopRange, score: &Score) {
        let playhead = {
            let mut state = self.state.write();
            state.loop_range = Some(range);
            if state.clock.is_some() {
                audio::set_loop(Some(loop_seconds(range, score)));
            }
            state.performance_timestamp
        };
        if !range.contains(playhead) {
            self.seek(range.start, score);
        }
    }

    pub fn clear_loop(&mut self) {
        if self.state.peek().loop_range.is_none() {
            return;
        }
        self.state.write().loop_range = None;
        audio::set_loop(None);
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
        if state.clock.is_some() {
            audio::set_speed(speed);
        }
        state.speed = speed;
    }

    pub fn volume(&self) -> f64 {
        (self.state)().volume
    }

    pub fn is_muted(&self) -> bool {
        (self.state)().muted
    }

    /// Set the volume, from 0 to 1. Turning it up unmutes
    pub fn set_volume(&mut self, volume: f64) {
        let mut state = self.state.write();
        state.volume = volume.clamp(0.0, 1.0);
        if state.volume > 0.0 {
            state.muted = false;
        }
        apply_volume(&state);
    }

    pub fn toggle_mute(&mut self) {
        let mut state = self.state.write();
        state.muted = !state.muted;
        apply_volume(&state);
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
        let loop_range = state.peek().loop_range;
        let start_timestamp = match loop_range {
            Some(range) if !range.contains(state.peek().performance_timestamp) => range.start,
            _ if state.peek().performance_timestamp >= end_timestamp => 0,
            _ => state.peek().performance_timestamp,
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
                    let seconds = clock.seconds(state.speed);
                    // Go round the loop, as the audio thread does
                    match state.loop_range {
                        Some(range) => {
                            let tempo_map = performance.tempo_map();
                            wrap_into_loop(
                                seconds,
                                tempo_map.ticks_to_seconds(range.start),
                                tempo_map.ticks_to_seconds(range.end),
                            )
                        }
                        None => seconds,
                    }
                };
                let timestamp = performance.tempo_map().seconds_to_ticks(seconds);

                let mut state = state.write();
                if timestamp >= f64::from(end_timestamp) {
                    // Finished: rewind so the next play starts from the top
                    state.clock = None;
                    audio::stop();
                    state.performance_timestamp = 0;
                    state.playhead_timestamp = 0;
                    break;
                }
                state.performance_timestamp = timestamp as u32;
                state.playhead_timestamp = performance.to_written(timestamp as u32);
            }
        });

        audio::play(
            audio::score_hits(score),
            start_seconds,
            state.peek().speed,
            loop_range.map(|range| loop_seconds(range, score)),
        );

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
            audio::stop();
        }
    }

    /// Stop playing and move the playhead back to the start, e.g. when a new
    /// score is opened
    pub fn reset(&mut self) {
        self.pause();
        let mut state = self.state.write();
        state.loop_range = None;
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

    pub fn skip_to_start(&mut self, score: &Score) {
        self.seek(0, score);
    }

    /// Move the playhead to the start of the last measure played
    pub fn skip_to_end(&mut self, score: &Score) {
        if let Some(last) = score.performance.measures().last() {
            self.seek(last.timestamp, score);
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

/// Where a loop starts and ends, in seconds into the performance at the
/// score's own tempo
fn loop_seconds(range: LoopRange, score: &Score) -> (f64, f64) {
    let tempo_map = score.performance.tempo_map();
    (
        tempo_map.ticks_to_seconds(range.start),
        tempo_map.ticks_to_seconds(range.end),
    )
}

fn apply_volume(state: &PlaybackState) {
    let volume = if state.muted { 0.0 } else { state.volume };
    audio::set_volume(volume as f32);
}

pub fn use_playback_manager() -> PlaybackManager {
    let playback_state = use_context::<Signal<PlaybackState>>();

    PlaybackManager::new(playback_state)
}

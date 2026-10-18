use std::time::{Duration, Instant};

use dioxus::{core::Task, prelude::*};

use crate::{
    audio::{self, ClickPosition, MetronomeSettings, midi, wrap_into_loop},
    data::{loop_range::LoopRange, score::Score},
    settings::{self, AudioSettings, DrumsOutput},
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
    /// Volume, mute and where the drums are played, kept between launches
    audio: AudioSettings,
    /// The section being practised, which plays over and over. The playhead
    /// is kept inside it
    loop_range: Option<LoopRange>,
    /// Counts every time the playhead is moved by hand, so the score can tell
    /// a jump from the playhead playing on to the next staff
    seeks: u32,
    /// Whether the last move by hand was made on the score itself, where the
    /// playhead is already in view
    last_seek_on_score: bool,
    /// Set while the score is playing
    clock: Option<PlaybackClock>,
    metronome: MetronomeSettings,
    /// Where the metronome's last click fell, as the audio thread played it
    click: Option<ClickPosition>,
}

/// Ties the playhead to real time: `start_seconds` into the performance (at
/// the score's own tempo) was reached at `started_at`. While counting in,
/// `start_seconds` is before where the score starts, at `floor_seconds`,
/// and the playhead waits there
#[derive(Clone, Copy)]
struct PlaybackClock {
    started_at: Instant,
    start_seconds: f64,
    floor_seconds: f64,
    task: Task,
}

impl PlaybackClock {
    /// Seconds into the performance at the score's own tempo
    fn seconds(&self, speed: f64) -> f64 {
        self.unclamped_seconds(speed).max(self.floor_seconds)
    }

    /// Seconds into the performance, counting back through the count-in
    fn unclamped_seconds(&self, speed: f64) -> f64 {
        self.start_seconds + self.started_at.elapsed().as_secs_f64() * speed
    }
}

pub fn use_playback_manager_provider() {
    use_context_provider(|| {
        Signal::new(PlaybackState {
            playhead_timestamp: 0,
            performance_timestamp: 0,
            speed: 1.0,
            audio: {
                let audio = settings::load().audio;
                apply_audio(&audio);
                connect_midi(&audio);
                audio
            },
            loop_range: None,
            seeks: 0,
            last_seek_on_score: false,
            clock: None,
            metronome: MetronomeSettings::default(),
            click: None,
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
        self.seek_from(timestamp, score, false);
    }

    /// Move the playhead to where it was clicked on the score
    pub fn seek_on_score(&mut self, timestamp: u32, score: &Score) {
        self.seek_from(timestamp, score, true);
    }

    fn seek_from(&mut self, timestamp: u32, score: &Score, on_score: bool) {
        let mut state = self.state.write();
        state.seeks = state.seeks.wrapping_add(1);
        state.last_seek_on_score = on_score;
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
            clock.floor_seconds = clock.start_seconds;
            audio::seek(clock.start_seconds);
        }
    }

    pub fn loop_range(&self) -> Option<LoopRange> {
        (self.state)().loop_range
    }

    /// Start looping `range`, moving the playhead to its start if it's
    /// outside it. `on_score` is whether the loop was picked out on the score
    pub fn set_loop(&mut self, range: LoopRange, score: &Score, on_score: bool) {
        let playhead = {
            let mut state = self.state.write();
            state.loop_range = Some(range);
            if state.clock.is_some() {
                audio::set_loop(Some(loop_seconds(range, score)));
            }
            state.performance_timestamp
        };
        if !range.contains(playhead) {
            self.seek_from(range.start, score, on_score);
        }
    }

    pub fn clear_loop(&mut self) {
        if self.state.peek().loop_range.is_none() {
            return;
        }
        self.state.write().loop_range = None;
        audio::set_loop(None);
    }

    /// How many times the playhead has been moved by hand, and whether the
    /// last time was on the score
    pub fn seeks(&self) -> (u32, bool) {
        let state = (self.state)();
        (state.seeks, state.last_seek_on_score)
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
            clock.start_seconds = clock.unclamped_seconds(old_speed);
            clock.started_at = Instant::now();
        }
        if state.clock.is_some() {
            audio::set_speed(speed);
        }
        state.speed = speed;
    }

    pub fn volume(&self) -> f64 {
        (self.state)().audio.volume
    }

    pub fn is_muted(&self) -> bool {
        (self.state)().audio.muted
    }

    /// Set the volume, from 0 to 1. Turning it up unmutes
    pub fn set_volume(&mut self, volume: f64) {
        self.update_audio(|audio| {
            audio.volume = volume.clamp(0.0, 1.0);
            if audio.volume > 0.0 {
                audio.muted = false;
            }
        });
    }

    pub fn toggle_mute(&mut self) {
        self.update_audio(|audio| audio.muted = !audio.muted);
    }

    pub fn drums_output(&self) -> DrumsOutput {
        (self.state)().audio.output
    }

    pub fn set_drums_output(&mut self, output: DrumsOutput) {
        self.update_audio(|audio| audio.output = output);
        connect_midi(&self.state.peek().audio);
    }

    pub fn midi_port(&self) -> String {
        (self.state)().audio.midi_port
    }

    pub fn set_midi_port(&mut self, port: String) {
        self.update_audio(|audio| audio.midi_port = port);
        connect_midi(&self.state.peek().audio);
    }

    /// Try the picked port again, e.g. once a device is plugged back in
    pub fn reconnect_midi(&self) {
        connect_midi(&self.state.peek().audio);
    }

    pub fn midi_offset_ms(&self) -> u32 {
        (self.state)().audio.midi_offset_ms
    }

    pub fn set_midi_offset_ms(&mut self, ms: u32) {
        self.update_audio(|audio| audio.midi_offset_ms = ms.min(audio::MAX_SEND_EARLY_MS));
    }

    /// Change the audio settings, passing them on to the audio and MIDI
    /// threads and saving them
    fn update_audio(&mut self, change: impl FnOnce(&mut AudioSettings)) {
        let mut state = self.state.write();
        change(&mut state.audio);
        apply_audio(&state.audio);
        settings::save_audio(&state.audio);
    }

    pub fn metronome(&self) -> MetronomeSettings {
        (self.state)().metronome
    }

    /// Change how the metronome plays along, straight away if the score is
    /// playing
    pub fn set_metronome(&mut self, settings: MetronomeSettings, score: &Score) {
        let mut state = self.state.write();
        audio::set_click_volume(settings.volume as f32);
        let old = &state.metronome;
        let clicks_changed = old.enabled != settings.enabled
            || old.sound != settings.sound
            || old.subdivision != settings.subdivision
            || old.accent_first != settings.accent_first;
        if state.clock.is_some() && clicks_changed {
            let clicks = if settings.enabled {
                audio::score_clicks(score, &settings)
            } else {
                vec![]
            };
            audio::set_clicks(clicks, settings.sound);
        }
        state.metronome = settings;
    }

    pub fn toggle_metronome(&mut self, score: &Score) {
        let mut settings = self.state.peek().metronome.clone();
        settings.enabled = !settings.enabled;
        self.set_metronome(settings, score);
    }

    /// Where the metronome's last click fell, while it's clicking or
    /// counting in
    pub fn click(&self) -> Option<ClickPosition> {
        (self.state)().click
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
                    state.click = None;
                    audio::stop();
                    state.performance_timestamp = 0;
                    state.playhead_timestamp = 0;
                    break;
                }
                // The beat is lit from the click that was heard
                state.click = audio::click_status();
                state.performance_timestamp = timestamp as u32;
                state.playhead_timestamp = performance.to_written(timestamp as u32);
            }
        });

        let metronome = state.peek().metronome.clone();
        let count_in = audio::count_in_clicks(score, start_timestamp, &metronome);
        // The playhead waits at the start while the count-in plays
        let count_in_seconds = count_in.first().map_or(0.0, |click| -click.seconds);
        audio::play(
            audio::score_hits(score),
            audio::Metronome {
                clicks: if metronome.enabled {
                    audio::score_clicks(score, &metronome)
                } else {
                    vec![]
                },
                count_in,
                sound: metronome.sound,
            },
            start_seconds,
            state.peek().speed,
            loop_range.map(|range| loop_seconds(range, score)),
        );

        let mut state = state.write();
        state.performance_timestamp = start_timestamp;
        state.playhead_timestamp = score.performance.to_written(start_timestamp);
        state.clock = Some(PlaybackClock {
            started_at: Instant::now(),
            start_seconds: start_seconds - count_in_seconds,
            floor_seconds: start_seconds,
            task,
        });
    }

    /// Stop moving the playhead, leaving it where it is
    pub fn pause(&mut self) {
        let mut state = self.state.write();
        if let Some(clock) = state.clock.take() {
            clock.task.cancel();
            audio::stop();
            state.click = None;
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

fn apply_audio(audio: &AudioSettings) {
    let volume = if audio.muted { 0.0 } else { audio.volume };
    audio::set_volume(volume as f32);
    audio::set_midi_out(audio.output == DrumsOutput::Midi);
    audio::set_midi_muted(audio.muted);
    audio::set_send_early_ms(audio.midi_offset_ms);
}

/// Open the MIDI port when the drums go out as MIDI, and close it when they
/// don't. Kept apart from `apply_audio`, since reopening a port isn't free
fn connect_midi(audio: &AudioSettings) {
    let port = (audio.output == DrumsOutput::Midi).then(|| audio.midi_port.clone());
    midi::connect(port);
}

pub fn use_playback_manager() -> PlaybackManager {
    let playback_state = use_context::<Signal<PlaybackState>>();

    PlaybackManager::new(playback_state)
}

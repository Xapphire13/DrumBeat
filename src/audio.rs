//! Plays the score's drums through a small synthesizer. The audio thread
//! keeps its own clock on the performance timeline, so hits land on exact
//! samples however the UI's frames are timed

use std::{
    sync::{
        OnceLock,
        atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    time::{Duration, Instant},
};

use cpal::{
    FromSample, SizedSample, StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};

mod click;
mod hits;
pub mod midi;
mod practice;
mod voice;

use click::ClickKind;
pub use click::{
    Click, ClickPosition, ClickSound, MetronomeSettings, Subdivision, count_in_clicks,
    score_clicks, status as click_status,
};
pub use hits::{Hit, score_hits};
pub use practice::{BeatKind, PracticeSettings, SILENT_BAR_EVERY, SpeedTrainer};
use practice::{Practice, PracticeClick};
use voice::{Sound, Voice};

/// The most voices that ring at once. The oldest is dropped past this
const MAX_VOICES: usize = 64;

/// How long a choked cymbal rings before it's grabbed, in seconds
const CHOKE_AFTER: f32 = 0.06;

/// Scales the mix down so several hits at once don't clip
const MASTER_GAIN: f32 = 0.5;

/// The output volume as `f32` bits, from 0 to 1. Kept outside the command
/// channel so setting it doesn't open the output device
static VOLUME: AtomicU32 = AtomicU32::new(0x3f80_0000); // 1.0

/// The metronome's volume over the score, kept apart from the music's
static CLICK_VOLUME: AtomicU32 = AtomicU32::new(0x3f4c_cccd); // 0.8

/// The practice metronome's volume
static PRACTICE_VOLUME: AtomicU32 = AtomicU32::new(0x3f4c_cccd); // 0.8

/// The tempo the practice metronome is playing at, which the speed trainer
/// raises as it goes: its settings' `tempo_id` in the high half and the BPM
/// as `f32` bits in the low
static PRACTICE_BPM: AtomicU64 = AtomicU64::new(0);

fn report_practice_bpm(tempo_id: u32, bpm: f64) {
    let bits = u64::from(tempo_id) << 32 | u64::from((bpm as f32).to_bits());
    PRACTICE_BPM.store(bits, Ordering::Relaxed);
}

/// Whether the score's drums go out as MIDI instead of to the built-in
/// drums. The metronome stays built in either way
static MIDI_OUT: AtomicBool = AtomicBool::new(false);

/// Whether MIDI notes are held back, the way mute silences the built-in drums
static MIDI_MUTED: AtomicBool = AtomicBool::new(false);

/// How far ahead of the playhead MIDI notes are sent, in milliseconds, to
/// make up for the delay of whatever plays them
static SEND_EARLY_MS: AtomicU32 = AtomicU32::new(0);

/// The most notes can be sent early by
pub const MAX_SEND_EARLY_MS: u32 = 100;

/// Send the score's drums out as MIDI, or play them on the built-in drums
pub fn set_midi_out(enabled: bool) {
    MIDI_OUT.store(enabled, Ordering::Relaxed);
}

pub fn set_midi_muted(muted: bool) {
    MIDI_MUTED.store(muted, Ordering::Relaxed);
}

pub fn set_send_early_ms(ms: u32) {
    SEND_EARLY_MS.store(ms.min(MAX_SEND_EARLY_MS), Ordering::Relaxed);
}

/// How far the volume moves towards a new setting each sample, so changing it
/// doesn't click
const VOLUME_SMOOTHING: f32 = 0.002;

enum Command {
    /// Start playing `hits` from `from_seconds` into the performance
    Play {
        hits: Vec<Hit>,
        metronome: Metronome,
        from_seconds: f64,
        speed: f64,
        loop_seconds: Option<(f64, f64)>,
    },
    Seek(f64),
    SetLoop(Option<(f64, f64)>),
    SetSpeed(f64),
    SetClicks {
        clicks: Vec<Click>,
        sound: ClickSound,
    },
    Stop,
    StartPractice(PracticeSettings),
    UpdatePractice(PracticeSettings),
    StopPractice,
}

/// The metronome's part in playing the score
#[derive(Default)]
pub struct Metronome {
    /// Clicks along with the score, sorted by time on the same timeline as
    /// its hits. Empty with the click off
    pub clicks: Vec<Click>,
    /// Clicks played before the score starts, timed in seconds (at the
    /// score's tempo) back from where it does, so all before 0
    pub count_in: Vec<Click>,
    pub sound: ClickSound,
}

/// Start playing `hits`, sorted by time, from `from_seconds` into the
/// performance at the score's own tempo, after the metronome's count-in.
/// `speed` multiplies that tempo. See `set_loop` for `loop_seconds`
pub fn play(
    hits: Vec<Hit>,
    metronome: Metronome,
    from_seconds: f64,
    speed: f64,
    loop_seconds: Option<(f64, f64)>,
) {
    send(Command::Play {
        hits,
        metronome,
        from_seconds,
        speed,
        loop_seconds,
    });
}

/// Change the metronome's clicks while the score plays. Empty turns it off
pub fn set_clicks(clicks: Vec<Click>, sound: ClickSound) {
    send(Command::SetClicks { clicks, sound });
}

/// Set the metronome's volume over the score, from 0 to 1
pub fn set_click_volume(volume: f32) {
    CLICK_VOLUME.store(volume.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
}

/// Start the practice metronome, which keeps its own time
pub fn start_practice(settings: PracticeSettings) {
    report_practice_bpm(settings.tempo_id, settings.bpm);
    send(Command::StartPractice(settings));
}

/// Change the practice metronome's settings while it plays, keeping the beat
pub fn update_practice(settings: PracticeSettings) {
    send(Command::UpdatePractice(settings));
}

pub fn stop_practice() {
    send(Command::StopPractice);
}

/// Set the practice metronome's volume, from 0 to 1
pub fn set_practice_volume(volume: f32) {
    PRACTICE_VOLUME.store(volume.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
}

/// The tempo the practice metronome is playing at, which the speed trainer
/// raises from the one it was given, with the `tempo_id` of the settings
/// that gave it. A report can lag a change of tempo by a click
pub fn practice_bpm() -> (u32, f64) {
    let bits = PRACTICE_BPM.load(Ordering::Relaxed);
    ((bits >> 32) as u32, f64::from(f32::from_bits(bits as u32)))
}

/// Jump to `seconds` into the performance, while playing
pub fn seek(seconds: f64) {
    send(Command::Seek(seconds));
}

/// Go back to the start of `(start, end)`, in seconds into the performance,
/// whenever playback reaches its end. The jump is made on the audio thread,
/// so the loop keeps time exactly
pub fn set_loop(loop_seconds: Option<(f64, f64)>) {
    send(Command::SetLoop(loop_seconds));
}

pub fn set_speed(speed: f64) {
    send(Command::SetSpeed(speed));
}

/// Set the output volume, from 0 (silent) to 1
pub fn set_volume(volume: f32) {
    VOLUME.store(volume.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
}

/// Stop playing, cutting off any hits that are still sounding
pub fn stop() {
    send(Command::Stop);
}

fn send(command: Command) {
    // Without an output device playback carries on silently
    if let Some(commands) = commands() {
        let _ = commands.send(command);
    }
}

/// The channel to the audio thread, which is started the first time it is
/// needed
fn commands() -> Option<&'static Sender<Command>> {
    static COMMANDS: OnceLock<Option<Sender<Command>>> = OnceLock::new();
    COMMANDS
        .get_or_init(|| match start() {
            Ok(commands) => Some(commands),
            Err(error) => {
                eprintln!("Audio is unavailable: {error}");
                None
            }
        })
        .as_ref()
}

/// Open the default output device on its own thread, which keeps the stream
/// alive for the rest of the app
fn start() -> Result<Sender<Command>, String> {
    let (commands, receiver) = mpsc::channel();
    let (ready, is_ready) = mpsc::channel();

    std::thread::Builder::new()
        .name("audio".into())
        .spawn(move || {
            let stream = match open_stream(receiver) {
                Ok(stream) => stream,
                Err(error) => {
                    let _ = ready.send(Err(error));
                    return;
                }
            };
            let _ = ready.send(Ok(()));
            // The stream stops when dropped, and it can't move between threads
            let _stream = stream;
            loop {
                std::thread::park();
            }
        })
        .map_err(|error| error.to_string())?;

    is_ready.recv().map_err(|error| error.to_string())??;
    Ok(commands)
}

fn open_stream(commands: Receiver<Command>) -> Result<cpal::Stream, String> {
    let device = cpal::default_host()
        .default_output_device()
        .ok_or("no output device")?;
    let config = device
        .default_output_config()
        .map_err(|error| error.to_string())?;

    match config.sample_format() {
        cpal::SampleFormat::F32 => build_stream::<f32>(&device, config.into(), commands),
        cpal::SampleFormat::I16 => build_stream::<i16>(&device, config.into(), commands),
        cpal::SampleFormat::U16 => build_stream::<u16>(&device, config.into(), commands),
        cpal::SampleFormat::I32 => build_stream::<i32>(&device, config.into(), commands),
        cpal::SampleFormat::F64 => build_stream::<f64>(&device, config.into(), commands),
        format => Err(format!("unsupported sample format {format}")),
    }
}

fn build_stream<T>(
    device: &cpal::Device,
    config: StreamConfig,
    commands: Receiver<Command>,
) -> Result<cpal::Stream, String>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = usize::from(config.channels);
    let mut mixer = Mixer::new(f64::from(config.sample_rate));

    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [T], info: &cpal::OutputCallbackInfo| {
                while let Ok(command) = commands.try_recv() {
                    mixer.handle(command);
                }
                // When this buffer's first sample will be heard
                let timestamp = info.timestamp();
                let latency = timestamp
                    .playback
                    .saturating_duration_since(timestamp.callback);
                mixer.start_buffer(Instant::now() + latency);
                for bus in mixer.buses_mut() {
                    bus.load_volume();
                }
                for frame in data.chunks_mut(channels) {
                    let sample = T::from_sample(mixer.next_sample());
                    frame.fill(sample);
                }
                for bus in mixer.buses_mut() {
                    bus.voices.retain(|voice| !voice.is_finished());
                }
            },
            |error| eprintln!("Audio stream error: {error}"),
            None,
        )
        .map_err(|error| error.to_string())?;
    stream.play().map_err(|error| error.to_string())?;
    Ok(stream)
}

/// Where `seconds` lands once it has gone round a loop from `start` to `end`
/// as many times as it has passed `end`. The UI's playhead uses the same
/// sum, so it stays in step with what is heard
pub fn wrap_into_loop(seconds: f64, start: f64, end: f64) -> f64 {
    if seconds < end || end <= start {
        return seconds;
    }
    start + (seconds - start).rem_euclid(end - start)
}

/// Voices mixed together under one volume
struct Bus {
    voices: Vec<Voice>,
    /// The gain applied to the bus, easing towards `target_volume`
    volume: f32,
    target_volume: f32,
    /// Where the volume is set from
    setting: &'static AtomicU32,
}

impl Bus {
    fn new(setting: &'static AtomicU32) -> Self {
        let volume = f32::from_bits(setting.load(Ordering::Relaxed));
        Self {
            voices: Vec::with_capacity(MAX_VOICES),
            volume,
            target_volume: volume,
            setting,
        }
    }

    fn load_volume(&mut self) {
        self.target_volume = f32::from_bits(self.setting.load(Ordering::Relaxed));
    }

    fn push(&mut self, voice: Voice) {
        if self.voices.len() >= MAX_VOICES {
            self.voices.remove(0);
        }
        self.voices.push(voice);
    }

    /// A quick fade on every voice rather than silence, which would click
    fn choke(&mut self) {
        for voice in &mut self.voices {
            voice.choke();
        }
    }

    fn next_sample(&mut self) -> f32 {
        let mix: f32 = self.voices.iter_mut().map(Voice::next_sample).sum();
        self.volume += (self.target_volume - self.volume) * VOLUME_SMOOTHING;
        // Squared, since loudness is heard on a roughly logarithmic scale
        (mix * MASTER_GAIN).tanh() * self.volume * self.volume
    }
}

/// The count-in before the score plays
struct CountIn {
    clicks: Vec<Click>,
    /// Index into `clicks` of the next one to play
    next_click: usize,
    /// Seconds (at the score's tempo) until the score starts, negative
    seconds: f64,
}

/// Runs on the audio thread: triggers hits as their time comes and mixes the
/// voices that are sounding
struct Mixer {
    sample_rate: f64,
    hits: Vec<Hit>,
    /// Index into `hits` of the next one to play
    next_hit: usize,
    /// The metronome's clicks over the score, on the same timeline as `hits`
    clicks: Vec<Click>,
    next_click: usize,
    click_sound: ClickSound,
    /// Set while counting in, when the score waits at `position`
    count_in: Option<CountIn>,
    /// Seconds into the performance at the score's own tempo
    position: f64,
    speed: f64,
    playing: bool,
    /// Where playback jumps back from and to, in seconds into the performance
    loop_seconds: Option<(f64, f64)>,
    music: Bus,
    /// The metronome over the score, and its count-in
    click_bus: Bus,
    practice_bus: Bus,
    /// Set while the practice metronome is going
    practice: Option<Practice>,
    /// Changes for every voice, so overlapping hits don't share their noise
    seed: u32,
    midi: MidiCursor,
    /// When the sample being worked out will be heard
    buffer_start: Instant,
    frames_into_buffer: u32,
    /// The times of the hits sent as MIDI, round the loop included
    #[cfg(test)]
    midi_sent: Vec<f64>,
}

/// Where MIDI out has got to in the score's hits. It runs ahead of the
/// playhead by the send-early time, so it has its own place in them
struct MidiCursor {
    enabled: bool,
    muted: bool,
    /// Seconds (at the score's tempo) notes are sent ahead of the playhead
    send_early: f64,
    /// Index into the hits of the next one to send
    next_hit: usize,
    /// Added to the hits' times once the cursor has gone round the loop
    /// ahead of the playhead
    lap_offset: f64,
}

impl Mixer {
    fn new(sample_rate: f64) -> Self {
        Self {
            sample_rate,
            hits: vec![],
            next_hit: 0,
            clicks: vec![],
            next_click: 0,
            click_sound: ClickSound::default(),
            count_in: None,
            position: 0.0,
            speed: 1.0,
            playing: false,
            loop_seconds: None,
            music: Bus::new(&VOLUME),
            click_bus: Bus::new(&CLICK_VOLUME),
            practice_bus: Bus::new(&PRACTICE_VOLUME),
            practice: None,
            seed: 1,
            midi: MidiCursor {
                enabled: false,
                muted: false,
                send_early: 0.0,
                next_hit: 0,
                lap_offset: 0.0,
            },
            buffer_start: Instant::now(),
            frames_into_buffer: 0,
            #[cfg(test)]
            midi_sent: vec![],
        }
    }

    /// Pick up MIDI out's settings for the buffer about to be worked out,
    /// whose first sample will be heard at `start`
    fn start_buffer(&mut self, start: Instant) {
        self.buffer_start = start;
        self.frames_into_buffer = 0;
        let enabled = MIDI_OUT.load(Ordering::Relaxed);
        if enabled && !self.midi.enabled {
            // Switched over mid-play: carry on from the playhead
            self.reset_midi_cursor(self.position);
        }
        if !enabled && self.midi.enabled {
            midi::all_notes_off();
        }
        self.midi.enabled = enabled;
        self.midi.muted = MIDI_MUTED.load(Ordering::Relaxed);
        self.midi.send_early = f64::from(SEND_EARLY_MS.load(Ordering::Relaxed)) / 1000.0;
    }

    fn reset_midi_cursor(&mut self, seconds: f64) {
        self.midi.next_hit = self.hits.partition_point(|hit| hit.seconds < seconds);
        self.midi.lap_offset = 0.0;
    }

    /// When the sample being worked out will be heard
    fn sample_time(&self) -> Instant {
        self.buffer_start
            + Duration::from_secs_f64(f64::from(self.frames_into_buffer) / self.sample_rate)
    }

    /// Send every hit that's due within the send-early time, timed to land
    /// that far ahead of when it's heard on the built-in drums
    fn schedule_midi(&mut self) {
        if !self.midi.enabled || self.speed <= 0.0 {
            return;
        }
        // While counting in, the score is that far off yet
        let now = self.position + self.count_in.as_ref().map_or(0.0, |c| c.seconds);
        let horizon = now + self.midi.send_early * self.speed;
        loop {
            if let Some(&hit) = self.hits.get(self.midi.next_hit)
                && self.loop_seconds.is_none_or(|(_, end)| hit.seconds < end)
            {
                let due = hit.seconds + self.midi.lap_offset;
                if due > horizon {
                    return;
                }
                self.midi.next_hit += 1;
                if !self.midi.muted {
                    // Never later than now, so this is at most the
                    // send-early time ahead of the sample
                    let early = self.midi.send_early - (due - now) / self.speed;
                    let at = self
                        .sample_time()
                        .checked_sub(Duration::from_secs_f64(early.max(0.0)))
                        .unwrap_or_else(Instant::now);
                    let velocity = (hit.velocity * 100.0).round() as u8;
                    #[cfg(test)]
                    self.midi_sent.push(due);
                    #[cfg(not(test))]
                    midi::schedule(at, hit.midi_note, velocity);
                    #[cfg(test)]
                    let _ = (at, velocity);
                }
                continue;
            }
            // Out of hits before the loop's end: go round it, if its end is
            // within reach
            match self.loop_seconds {
                Some((start, end)) if end > start && end + self.midi.lap_offset <= horizon => {
                    self.midi.lap_offset += end - start;
                    self.midi.next_hit = self.hits.partition_point(|hit| hit.seconds < start);
                }
                _ => return,
            }
        }
    }

    fn buses_mut(&mut self) -> [&mut Bus; 3] {
        [&mut self.music, &mut self.click_bus, &mut self.practice_bus]
    }

    fn handle(&mut self, command: Command) {
        match command {
            Command::Play {
                hits,
                metronome,
                from_seconds,
                speed,
                loop_seconds,
            } => {
                self.hits = hits;
                self.clicks = metronome.clicks;
                self.click_sound = metronome.sound;
                self.speed = speed;
                self.loop_seconds = loop_seconds;
                self.playing = true;
                self.seek(from_seconds);
                if let Some(first) = metronome.count_in.first() {
                    self.count_in = Some(CountIn {
                        seconds: first.seconds,
                        clicks: metronome.count_in,
                        next_click: 0,
                    });
                }
            }
            Command::Seek(seconds) => self.seek(seconds),
            Command::SetLoop(loop_seconds) => {
                self.loop_seconds = loop_seconds;
                if self.midi.enabled {
                    midi::clear_pending();
                    self.reset_midi_cursor(self.position);
                }
            }
            Command::SetSpeed(speed) => self.speed = speed,
            Command::SetClicks { clicks, sound } => {
                self.clicks = clicks;
                self.click_sound = sound;
                self.next_click = self.clicks.partition_point(|c| c.seconds < self.position);
                if self.clicks.is_empty() && self.count_in.is_none() {
                    click::publish(None);
                }
            }
            Command::Stop => {
                if self.midi.enabled {
                    midi::all_notes_off();
                }
                self.playing = false;
                self.count_in = None;
                click::publish(None);
                self.music.choke();
                self.click_bus.choke();
            }
            Command::StartPractice(settings) => {
                self.practice = Some(Practice::new(settings));
            }
            Command::UpdatePractice(settings) => {
                if let Some(practice) = &mut self.practice {
                    practice.update(settings);
                }
            }
            Command::StopPractice => {
                self.practice = None;
                click::publish(None);
                self.practice_bus.choke();
            }
        }
    }

    fn seek(&mut self, seconds: f64) {
        self.position = seconds;
        if self.midi.enabled {
            midi::clear_pending();
        }
        self.reset_midi_cursor(seconds);
        // A hit right on the new position is played
        self.next_hit = self.hits.partition_point(|hit| hit.seconds < seconds);
        self.next_click = self.clicks.partition_point(|click| click.seconds < seconds);
        // Moving the playhead cuts the count-in short
        if self.count_in.take().is_some() && self.clicks.is_empty() {
            click::publish(None);
        }
    }

    /// The next hit whose time has come, if any, which is then counted as
    /// played
    fn next_due_hit(&mut self) -> Option<Hit> {
        let hit = *self.hits.get(self.next_hit)?;
        if hit.seconds > self.position {
            return None;
        }
        self.next_hit += 1;
        Some(hit)
    }

    fn next_due_click(&mut self) -> Option<Click> {
        let click = *self.clicks.get(self.next_click)?;
        if click.seconds > self.position {
            return None;
        }
        self.next_click += 1;
        Some(click)
    }

    /// Move on by one sample, going round the loop at its end
    fn advance(&mut self) {
        self.position += self.speed / self.sample_rate;
        if let Some((start, end)) = self.loop_seconds
            && self.position >= end
        {
            let wrapped = wrap_into_loop(self.position, start, end);
            // MIDI out is already round the loop, so it's that much less
            // ahead now
            self.midi.lap_offset -= self.position - wrapped;
            self.position = wrapped;
            // The new position is a fraction of a sample past the start, so
            // a plain seek would skip the hits right on it, dropping the
            // loop's first note every time round
            self.next_hit = self.hits.partition_point(|hit| hit.seconds < start);
            self.next_click = self.clicks.partition_point(|click| click.seconds < start);
        }
    }

    /// Move the count-in on by one sample, playing its clicks as they come
    /// and ending it once the score's time comes
    fn count_in(&mut self) {
        let Some(count_in) = &mut self.count_in else {
            return;
        };
        let mut due = None;
        if let Some(click) = count_in.clicks.get(count_in.next_click)
            && click.seconds <= count_in.seconds
        {
            count_in.next_click += 1;
            due = Some(*click);
        }
        count_in.seconds += self.speed / self.sample_rate;
        let finished = count_in.seconds >= 0.0;
        if let Some(click) = due {
            self.trigger_click(click);
        }
        if finished {
            self.count_in = None;
            if self.clicks.is_empty() {
                click::publish(None);
            }
        }
    }

    fn next_sample(&mut self) -> f32 {
        if self.playing {
            self.schedule_midi();
            if self.count_in.is_some() {
                self.count_in();
            } else {
                while let Some(hit) = self.next_due_hit() {
                    self.trigger(hit);
                }
                while let Some(click) = self.next_due_click() {
                    self.trigger_click(click);
                }
                self.advance();
            }
        }

        if let Some(practice) = &mut self.practice
            && let Some(click) = practice.tick(self.sample_rate)
        {
            report_practice_bpm(practice.tempo_id(), practice.bpm());
            self.trigger_practice(click);
        }

        self.frames_into_buffer += 1;
        let mix = self.music.next_sample()
            + self.click_bus.next_sample()
            + self.practice_bus.next_sample();
        mix.clamp(-1.0, 1.0)
    }

    fn next_seed(&mut self) -> u32 {
        self.seed = self
            .seed
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        self.seed
    }

    fn trigger(&mut self, hit: Hit) {
        // MIDI out sends the drums on its own clock
        if self.midi.enabled {
            return;
        }
        if hit.sound.is_hihat() {
            // A new stroke on the hi-hat stops it ringing open
            for voice in &mut self.music.voices {
                if voice.sound().is_hihat() {
                    voice.choke();
                }
            }
        }
        if hit.choke {
            // Grabbing the cymbal stops it ringing from before too
            for voice in &mut self.music.voices {
                if voice.sound() == hit.sound {
                    voice.choke();
                }
            }
        }
        let seed = self.next_seed();
        let mut voice = Voice::new(hit.sound, hit.velocity, self.sample_rate as f32, seed);
        if hit.choke {
            voice.choke_after(CHOKE_AFTER);
        }
        self.music.push(voice);
    }

    fn trigger_click(&mut self, click: Click) {
        click::publish(Some(click.position));
        let voice = self.click_voice(self.click_sound, click.kind);
        self.click_bus.push(voice);
    }

    fn trigger_practice(&mut self, click: PracticeClick) {
        click::publish(Some(click.position));
        let sound = self.practice.as_ref().and_then(Practice::sound);
        if let (Some(kind), Some(sound)) = (click.kind, sound) {
            let voice = self.click_voice(sound, kind);
            self.practice_bus.push(voice);
        }
    }

    fn click_voice(&mut self, sound: ClickSound, kind: ClickKind) -> Voice {
        let seed = self.next_seed();
        Voice::new(
            Sound::Click {
                sound,
                accent: kind == ClickKind::Accent,
            },
            kind.velocity(),
            self.sample_rate as f32,
            seed,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::voice::Sound;

    fn hit(seconds: f64) -> Hit {
        Hit {
            seconds,
            sound: Sound::Snare,
            choke: false,
            midi_note: 38,
            velocity: 1.0,
        }
    }

    /// Run the mixer's clock for `samples`, returning the times hits were
    /// played at
    fn run(mixer: &mut Mixer, samples: usize) -> Vec<f64> {
        let mut played = vec![];
        for _ in 0..samples {
            while let Some(hit) = mixer.next_due_hit() {
                played.push(hit.seconds);
            }
            mixer.advance();
        }
        played
    }

    #[test]
    fn loop_plays_its_first_note_every_time_round() {
        // 1000 samples a second, so the loop is 750 samples long
        let mut mixer = Mixer::new(1000.0);
        mixer.handle(Command::Play {
            hits: vec![hit(0.0), hit(0.25), hit(0.5), hit(1.0)],
            metronome: Metronome::default(),
            from_seconds: 0.25,
            speed: 1.0,
            loop_seconds: Some((0.25, 1.0)),
        });

        let played = run(&mut mixer, 2250);
        // The hit at the loop's end is never reached, and the one at its
        // start is played each time round
        assert_eq!(played, [0.25, 0.5, 0.25, 0.5, 0.25, 0.5]);
    }

    #[test]
    fn midi_runs_ahead_round_the_loop() {
        // 1000 samples a second, sending 100ms early
        let mut mixer = Mixer::new(1000.0);
        mixer.handle(Command::Play {
            hits: vec![hit(0.0), hit(0.25), hit(0.5), hit(1.0)],
            metronome: Metronome::default(),
            from_seconds: 0.25,
            speed: 1.0,
            loop_seconds: Some((0.25, 1.0)),
        });
        mixer.midi.enabled = true;
        mixer.midi.send_early = 0.1;
        mixer.reset_midi_cursor(0.25);

        let mut sent_at = vec![];
        for _ in 0..1600 {
            let before = mixer.midi_sent.len();
            mixer.next_sample();
            if mixer.midi_sent.len() > before {
                sent_at.push(mixer.position);
            }
        }
        // Each hit is sent 100ms before the playhead reaches it. The loop's
        // first hit is sent while the playhead is still near the end, so it's
        // due a lap on (at 1.0), and the hit at the loop's end is never sent
        assert_eq!(mixer.midi_sent, [0.25, 0.5, 1.0, 0.5, 1.0]);
        let expected = [0.25, 0.4, 0.9, 0.4, 0.9];
        assert_eq!(sent_at.len(), expected.len());
        for (sent, expected) in sent_at.iter().zip(expected) {
            assert!((sent - expected).abs() < 0.002, "{sent_at:?}");
        }
        // The built-in drums stay quiet
        assert!(mixer.music.voices.is_empty());
    }

    #[test]
    fn count_in_holds_the_score_back() {
        let click = |seconds| Click {
            seconds,
            kind: ClickKind::Beat,
            position: ClickPosition::default(),
        };
        // 1000 samples a second, so the count-in is 500 samples long
        let mut mixer = Mixer::new(1000.0);
        mixer.handle(Command::Play {
            hits: vec![hit(0.5), hit(1.0)],
            metronome: Metronome {
                count_in: vec![click(-0.5), click(-0.25)],
                ..Metronome::default()
            },
            from_seconds: 1.0,
            speed: 1.0,
            loop_seconds: None,
        });

        for _ in 0..495 {
            mixer.next_sample();
        }
        assert_eq!(mixer.click_bus.voices.len(), 2);
        assert!(mixer.music.voices.is_empty());
        for _ in 0..10 {
            mixer.next_sample();
        }
        // Only the hit where playback starts, not the one before it
        assert_eq!(mixer.music.voices.len(), 1);
    }

    #[test]
    fn wrap_keeps_time() {
        // A loop that isn't a whole number of samples long
        let mut mixer = Mixer::new(1000.0);
        mixer.handle(Command::Play {
            hits: vec![],
            metronome: Metronome::default(),
            from_seconds: 0.0,
            speed: 1.0,
            loop_seconds: Some((0.0, 0.0105)),
        });
        run(&mut mixer, 100);
        // 100 samples is 0.1s, 9 times round with 0.0055s left over
        assert!((mixer.position - 0.0055).abs() < 1e-9);
        assert!((wrap_into_loop(0.1, 0.0, 0.0105) - 0.0055).abs() < 1e-9);
    }
}

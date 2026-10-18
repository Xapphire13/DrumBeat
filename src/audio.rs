//! Plays the score's drums through a small synthesizer. The audio thread
//! keeps its own clock on the performance timeline, so hits land on exact
//! samples however the UI's frames are timed

use std::sync::{
    OnceLock,
    atomic::{AtomicU32, AtomicU64, Ordering},
    mpsc::{self, Receiver, Sender},
};

use cpal::{
    FromSample, SizedSample, StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};

mod click;
mod hits;
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

/// Scales the mix down so several hits at once don't clip
const MASTER_GAIN: f32 = 0.5;

/// The output volume as `f32` bits, from 0 to 1. Kept outside the command
/// channel so setting it doesn't open the output device
static VOLUME: AtomicU32 = AtomicU32::new(0x3f80_0000); // 1.0

/// The metronome's volume over the score, kept apart from the music's
static CLICK_VOLUME: AtomicU32 = AtomicU32::new(0x3f4c_cccd); // 0.8

/// The practice metronome's volume
static PRACTICE_VOLUME: AtomicU32 = AtomicU32::new(0x3f4c_cccd); // 0.8

/// The tempo the practice metronome is playing at, as `f64` bits, which the
/// speed trainer raises as it goes
static PRACTICE_BPM: AtomicU64 = AtomicU64::new(0);

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
    PRACTICE_BPM.store(settings.bpm.to_bits(), Ordering::Relaxed);
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
/// raises from the one it was started with
pub fn practice_bpm() -> f64 {
    f64::from_bits(PRACTICE_BPM.load(Ordering::Relaxed))
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
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                while let Ok(command) = commands.try_recv() {
                    mixer.handle(command);
                }
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
            Command::SetLoop(loop_seconds) => self.loop_seconds = loop_seconds,
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
            self.position = wrap_into_loop(self.position, start, end);
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
            PRACTICE_BPM.store(practice.bpm().to_bits(), Ordering::Relaxed);
            self.trigger_practice(click);
        }

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
        if hit.sound.is_hihat() {
            // A new stroke on the hi-hat stops it ringing open
            for voice in &mut self.music.voices {
                if voice.sound().is_hihat() {
                    voice.choke();
                }
            }
        }
        let seed = self.next_seed();
        self.music.push(Voice::new(
            hit.sound,
            hit.velocity,
            self.sample_rate as f32,
            seed,
        ));
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

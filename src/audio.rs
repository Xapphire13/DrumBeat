//! Plays the score's drums through a small synthesizer. The audio thread
//! keeps its own clock on the performance timeline, so hits land on exact
//! samples however the UI's frames are timed

use std::sync::{
    OnceLock,
    atomic::{AtomicU32, Ordering},
    mpsc::{self, Receiver, Sender},
};

use cpal::{
    FromSample, SizedSample, StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};

mod hits;
mod voice;

pub use hits::{Hit, score_hits};
use voice::Voice;

/// The most voices that ring at once. The oldest is dropped past this
const MAX_VOICES: usize = 64;

/// Scales the mix down so several hits at once don't clip
const MASTER_GAIN: f32 = 0.5;

/// The output volume as `f32` bits, from 0 to 1. Kept outside the command
/// channel so setting it doesn't open the output device
static VOLUME: AtomicU32 = AtomicU32::new(0x3f80_0000); // 1.0

/// How far the volume moves towards a new setting each sample, so changing it
/// doesn't click
const VOLUME_SMOOTHING: f32 = 0.002;

enum Command {
    /// Start playing `hits` from `from_seconds` into the performance
    Play {
        hits: Vec<Hit>,
        from_seconds: f64,
        speed: f64,
        loop_seconds: Option<(f64, f64)>,
    },
    Seek(f64),
    SetLoop(Option<(f64, f64)>),
    SetSpeed(f64),
    Stop,
}

/// Start playing `hits`, sorted by time, from `from_seconds` into the
/// performance at the score's own tempo. `speed` multiplies that tempo.
/// See `set_loop` for `loop_seconds`
pub fn play(hits: Vec<Hit>, from_seconds: f64, speed: f64, loop_seconds: Option<(f64, f64)>) {
    send(Command::Play {
        hits,
        from_seconds,
        speed,
        loop_seconds,
    });
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
                mixer.target_volume = f32::from_bits(VOLUME.load(Ordering::Relaxed));
                for frame in data.chunks_mut(channels) {
                    let sample = T::from_sample(mixer.next_sample());
                    frame.fill(sample);
                }
                mixer.voices.retain(|voice| !voice.is_finished());
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

/// Runs on the audio thread: triggers hits as their time comes and mixes the
/// voices that are sounding
struct Mixer {
    sample_rate: f64,
    hits: Vec<Hit>,
    /// Index into `hits` of the next one to play
    next_hit: usize,
    /// Seconds into the performance at the score's own tempo
    position: f64,
    speed: f64,
    playing: bool,
    /// Where playback jumps back from and to, in seconds into the performance
    loop_seconds: Option<(f64, f64)>,
    voices: Vec<Voice>,
    /// Changes for every voice, so overlapping hits don't share their noise
    seed: u32,
    /// The gain applied to the mix, easing towards `target_volume`
    volume: f32,
    target_volume: f32,
}

impl Mixer {
    fn new(sample_rate: f64) -> Self {
        Self {
            sample_rate,
            hits: vec![],
            next_hit: 0,
            position: 0.0,
            speed: 1.0,
            playing: false,
            loop_seconds: None,
            voices: Vec::with_capacity(MAX_VOICES),
            seed: 1,
            volume: 1.0,
            target_volume: 1.0,
        }
    }

    fn handle(&mut self, command: Command) {
        match command {
            Command::Play {
                hits,
                from_seconds,
                speed,
                loop_seconds,
            } => {
                self.hits = hits;
                self.speed = speed;
                self.loop_seconds = loop_seconds;
                self.playing = true;
                self.seek(from_seconds);
            }
            Command::Seek(seconds) => self.seek(seconds),
            Command::SetLoop(loop_seconds) => self.loop_seconds = loop_seconds,
            Command::SetSpeed(speed) => self.speed = speed,
            Command::Stop => {
                self.playing = false;
                // A quick fade rather than silence, which would click
                for voice in &mut self.voices {
                    voice.choke();
                }
            }
        }
    }

    fn seek(&mut self, seconds: f64) {
        self.position = seconds;
        // A hit right on the new position is played
        self.next_hit = self.hits.partition_point(|hit| hit.seconds < seconds);
    }

    fn next_sample(&mut self) -> f32 {
        if self.playing {
            while let Some(&hit) = self.hits.get(self.next_hit)
                && hit.seconds <= self.position
            {
                self.trigger(hit);
                self.next_hit += 1;
            }
            self.position += self.speed / self.sample_rate;
            if let Some((start, end)) = self.loop_seconds
                && self.position >= end
            {
                self.seek(wrap_into_loop(self.position, start, end));
            }
        }

        let mix: f32 = self.voices.iter_mut().map(Voice::next_sample).sum();
        self.volume += (self.target_volume - self.volume) * VOLUME_SMOOTHING;
        // Squared, since loudness is heard on a roughly logarithmic scale
        (mix * MASTER_GAIN).tanh() * self.volume * self.volume
    }

    fn trigger(&mut self, hit: Hit) {
        if hit.sound.is_hihat() {
            // A new stroke on the hi-hat stops it ringing open
            for voice in &mut self.voices {
                if voice.sound().is_hihat() {
                    voice.choke();
                }
            }
        }
        if self.voices.len() >= MAX_VOICES {
            self.voices.remove(0);
        }
        self.seed = self
            .seed
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        self.voices.push(Voice::new(
            hit.sound,
            hit.velocity,
            self.sample_rate as f32,
            self.seed,
        ));
    }
}

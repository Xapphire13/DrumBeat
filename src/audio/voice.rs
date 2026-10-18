use std::f32::consts::TAU;

use crate::data::{drum_piece::DrumPiece, note::Articulation};

/// A synthesized drum sound
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sound {
    Kick,
    Snare,
    Tom1,
    Tom2,
    FloorTom,
    HihatClosed,
    HihatHalfOpen,
    HihatOpen,
    HihatPedal,
    Crash,
    Ride,
    RideBell,
    China,
    Splash,
}

impl Sound {
    pub fn new(piece: DrumPiece, articulation: Articulation) -> Self {
        match piece {
            DrumPiece::Kick => Sound::Kick,
            DrumPiece::Snare => Sound::Snare,
            DrumPiece::Tom1 => Sound::Tom1,
            DrumPiece::Tom2 => Sound::Tom2,
            DrumPiece::FloorTom => Sound::FloorTom,
            DrumPiece::Hihat => match articulation {
                Articulation::HihatOpen => Sound::HihatOpen,
                Articulation::HihatHalfOpen => Sound::HihatHalfOpen,
                _ => Sound::HihatClosed,
            },
            DrumPiece::HihatPedal => Sound::HihatPedal,
            DrumPiece::Crash => Sound::Crash,
            DrumPiece::Ride if articulation == Articulation::RideBell => Sound::RideBell,
            DrumPiece::Ride => Sound::Ride,
            DrumPiece::China => Sound::China,
            DrumPiece::Splash => Sound::Splash,
        }
    }

    /// Whether the sound is played on the hi-hat, so a new hi-hat hit cuts
    /// it off the way closing the cymbals would
    pub fn is_hihat(self) -> bool {
        matches!(
            self,
            Sound::HihatClosed | Sound::HihatHalfOpen | Sound::HihatOpen | Sound::HihatPedal
        )
    }

    /// How loud the sound is relative to the others
    fn level(self) -> f32 {
        match self {
            Sound::Kick => 1.0,
            Sound::Snare => 0.8,
            Sound::Tom1 | Sound::Tom2 | Sound::FloorTom => 0.8,
            Sound::HihatClosed | Sound::HihatHalfOpen | Sound::HihatOpen => 0.45,
            Sound::HihatPedal => 0.35,
            Sound::Crash | Sound::China => 0.5,
            Sound::Splash => 0.45,
            Sound::Ride => 0.35,
            Sound::RideBell => 0.4,
        }
    }

    /// Time constant of the main decay, in seconds
    fn decay(self) -> f32 {
        match self {
            Sound::Kick => 0.28,
            Sound::Snare => 0.14,
            Sound::Tom1 => 0.25,
            Sound::Tom2 => 0.3,
            Sound::FloorTom => 0.35,
            Sound::HihatClosed => 0.035,
            Sound::HihatPedal => 0.025,
            Sound::HihatHalfOpen => 0.12,
            Sound::HihatOpen => 0.35,
            Sound::Crash => 0.9,
            Sound::China => 0.7,
            Sound::Splash => 0.35,
            Sound::Ride => 0.6,
            Sound::RideBell => 0.5,
        }
    }
}

/// Frequencies of the square waves a TR-808 mixes for its cymbals. Played
/// together they sound metallic rather than like a chord
const METAL_FREQUENCIES: [f32; 6] = [205.3, 304.4, 369.6, 522.7, 540.0, 800.0];

/// How long a choked voice takes to fade out, in seconds
const CHOKE_DECAY: f32 = 0.008;

/// One drum hit being played
pub struct Voice {
    sound: Sound,
    gain: f32,
    sample_rate: f32,
    /// Seconds since the hit
    time: f32,
    /// Phase of the sine oscillator, from 0 to 1
    phase: f32,
    metal_phases: [f32; 6],
    noise: Noise,
    filter: Biquad,
    /// When the voice was cut off, in seconds since the hit
    choked_at: Option<f32>,
}

impl Voice {
    pub fn new(sound: Sound, velocity: f32, sample_rate: f32, seed: u32) -> Self {
        let filter = match sound {
            Sound::Snare => Biquad::highpass(sample_rate, 1200.0, 0.7),
            Sound::HihatClosed | Sound::HihatHalfOpen | Sound::HihatOpen | Sound::HihatPedal => {
                Biquad::highpass(sample_rate, 7000.0, 0.7)
            }
            Sound::Crash => Biquad::highpass(sample_rate, 4000.0, 0.7),
            Sound::Splash => Biquad::highpass(sample_rate, 5500.0, 0.7),
            Sound::China => Biquad::bandpass(sample_rate, 3000.0, 1.0),
            Sound::Ride => Biquad::bandpass(sample_rate, 5000.0, 0.8),
            Sound::RideBell => Biquad::bandpass(sample_rate, 2500.0, 2.0),
            // Unused by the drums
            Sound::Kick | Sound::Tom1 | Sound::Tom2 | Sound::FloorTom => Biquad::passthrough(),
        };

        Self {
            sound,
            gain: sound.level() * velocity,
            sample_rate,
            time: 0.0,
            phase: 0.0,
            metal_phases: [0.0; 6],
            noise: Noise::new(seed),
            filter,
            choked_at: None,
        }
    }

    pub fn sound(&self) -> Sound {
        self.sound
    }

    /// Fade the voice out quickly, e.g. an open hi-hat when it closes
    pub fn choke(&mut self) {
        self.choked_at.get_or_insert(self.time);
    }

    /// Whether the voice has faded to silence
    pub fn is_finished(&self) -> bool {
        match self.choked_at {
            Some(choked_at) => self.time - choked_at > CHOKE_DECAY * 7.0,
            None => self.time > self.sound.decay() * 7.0,
        }
    }

    pub fn next_sample(&mut self) -> f32 {
        let t = self.time;
        self.time += 1.0 / self.sample_rate;

        let out = match self.sound {
            Sound::Kick => {
                // A sine that drops quickly in pitch, with a click on the beater
                let tone = self.sine(45.0 + 110.0 * decay(t, 0.035)) * decay(t, 0.28);
                let click = self.noise.next() * decay(t, 0.002) * 0.4;
                tone + click
            }
            Sound::Snare => {
                let tone = self.sine(190.0 * (1.0 + 0.4 * decay(t, 0.012))) * decay(t, 0.07) * 0.6;
                let wires = self.filter.process(self.noise.next()) * decay(t, 0.14) * 0.8;
                tone + wires
            }
            Sound::Tom1 | Sound::Tom2 | Sound::FloorTom => {
                let pitch = match self.sound {
                    Sound::Tom1 => 200.0,
                    Sound::Tom2 => 150.0,
                    _ => 100.0,
                };
                let tone = self.sine(pitch * (1.0 + 0.5 * decay(t, 0.05)));
                let stick = self.noise.next() * decay(t, 0.004) * 0.2;
                tone * decay(t, self.sound.decay()) + stick
            }
            _ => {
                // Cymbals: metallic squares and noise, filtered to the
                // cymbal's range
                let (pitch, metal_mix) = match self.sound {
                    Sound::China => (0.8, 0.7),
                    Sound::Splash => (1.3, 0.4),
                    Sound::Ride => (1.1, 0.6),
                    Sound::RideBell => (1.6, 0.9),
                    Sound::Crash => (1.0, 0.4),
                    // Hi-hats
                    _ => (1.0, 0.5),
                };
                let metal = self.metal(pitch);
                let noise = self.noise.next();
                let mixed = metal * metal_mix + noise * (1.0 - metal_mix);
                let mut envelope = decay(t, self.sound.decay());
                if self.sound == Sound::Ride {
                    // The stick's ping over the wash
                    envelope = 0.6 * envelope + 0.4 * decay(t, 0.05);
                }
                self.filter.process(mixed) * envelope
            }
        };

        let choke = self
            .choked_at
            .map_or(1.0, |choked_at| decay(t - choked_at, CHOKE_DECAY));
        out * self.gain * choke
    }

    fn sine(&mut self, frequency: f32) -> f32 {
        self.phase = (self.phase + frequency / self.sample_rate).fract();
        (self.phase * TAU).sin()
    }

    /// The 808's six square waves, mixed
    fn metal(&mut self, pitch: f32) -> f32 {
        let mut out = 0.0;
        for (phase, frequency) in self.metal_phases.iter_mut().zip(METAL_FREQUENCIES) {
            *phase = (*phase + frequency * pitch / self.sample_rate).fract();
            out += if *phase < 0.5 { 1.0 } else { -1.0 };
        }
        out / METAL_FREQUENCIES.len() as f32
    }
}

/// Exponential decay from 1, falling to about a third every `time_constant`
/// seconds
fn decay(t: f32, time_constant: f32) -> f32 {
    (-t / time_constant).exp()
}

/// White noise from a xorshift generator
struct Noise(u32);

impl Noise {
    fn new(seed: u32) -> Self {
        // Xorshift gets stuck at zero
        Self(seed.max(1))
    }

    /// A sample between -1 and 1
    fn next(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x as f32 / u32::MAX as f32 * 2.0 - 1.0
    }
}

/// A second-order filter, from the Audio EQ Cookbook
struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl Biquad {
    fn new(b: [f32; 3], a: [f32; 3]) -> Self {
        Self {
            b0: b[0] / a[0],
            b1: b[1] / a[0],
            b2: b[2] / a[0],
            a1: a[1] / a[0],
            a2: a[2] / a[0],
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    fn passthrough() -> Self {
        Self::new([1.0, 0.0, 0.0], [1.0, 0.0, 0.0])
    }

    fn highpass(sample_rate: f32, frequency: f32, q: f32) -> Self {
        let (cos, alpha) = Self::coefficients(sample_rate, frequency, q);
        Self::new(
            [(1.0 + cos) / 2.0, -(1.0 + cos), (1.0 + cos) / 2.0],
            [1.0 + alpha, -2.0 * cos, 1.0 - alpha],
        )
    }

    /// A band-pass with 0 dB gain at its centre
    fn bandpass(sample_rate: f32, frequency: f32, q: f32) -> Self {
        let (cos, alpha) = Self::coefficients(sample_rate, frequency, q);
        Self::new([alpha, 0.0, -alpha], [1.0 + alpha, -2.0 * cos, 1.0 - alpha])
    }

    fn coefficients(sample_rate: f32, frequency: f32, q: f32) -> (f32, f32) {
        // Keep the filter stable at low sample rates
        let frequency = frequency.min(sample_rate * 0.45);
        let w0 = TAU * frequency / sample_rate;
        (w0.cos(), w0.sin() / (2.0 * q))
    }

    fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: f32 = 48_000.0;

    /// Every sound makes noise, stays in range and finishes
    #[test]
    fn sounds_are_audible_and_finish() {
        let sounds = [
            Sound::Kick,
            Sound::Snare,
            Sound::Tom1,
            Sound::Tom2,
            Sound::FloorTom,
            Sound::HihatClosed,
            Sound::HihatHalfOpen,
            Sound::HihatOpen,
            Sound::HihatPedal,
            Sound::Crash,
            Sound::Ride,
            Sound::RideBell,
            Sound::China,
            Sound::Splash,
        ];
        for sound in sounds {
            let mut voice = Voice::new(sound, 1.0, SAMPLE_RATE, 1);
            let mut peak: f32 = 0.0;
            let mut samples = 0;
            while !voice.is_finished() {
                let sample = voice.next_sample();
                assert!(sample.is_finite(), "{sound:?}");
                peak = peak.max(sample.abs());
                samples += 1;
                assert!(samples < SAMPLE_RATE as usize * 10, "{sound:?} never ends");
            }
            assert!(peak > 0.05, "{sound:?} is too quiet: {peak}");
            assert!(peak < 2.0, "{sound:?} is too loud: {peak}");
        }
    }

    #[test]
    fn choking_cuts_a_voice_short() {
        let mut voice = Voice::new(Sound::HihatOpen, 1.0, SAMPLE_RATE, 1);
        for _ in 0..100 {
            voice.next_sample();
        }
        voice.choke();
        let mut samples = 0;
        while !voice.is_finished() {
            voice.next_sample();
            samples += 1;
        }
        assert!(samples < (SAMPLE_RATE * 0.1) as usize);
    }
}

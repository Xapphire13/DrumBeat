use std::time::{Duration, Instant};

use dioxus::prelude::*;

use crate::audio::{
    self, BeatKind, ClickPosition, ClickSound, PracticeSettings, SpeedTrainer, Subdivision,
};

/// How often the beat's lights are brought up to date with the audio thread
const POLL_INTERVAL: Duration = Duration::from_millis(16);

pub const MIN_BPM: u32 = 30;
pub const MAX_BPM: u32 = 250;

/// The time signatures the practice metronome offers
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Meter {
    TwoFour,
    ThreeFour,
    FourFour,
    FiveFour,
    SixEight,
    SevenEight,
}

impl Meter {
    pub const ALL: [Meter; 6] = [
        Meter::TwoFour,
        Meter::ThreeFour,
        Meter::FourFour,
        Meter::FiveFour,
        Meter::SixEight,
        Meter::SevenEight,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Meter::TwoFour => "2/4",
            Meter::ThreeFour => "3/4",
            Meter::FourFour => "4/4",
            Meter::FiveFour => "5/4",
            Meter::SixEight => "6/8",
            Meter::SevenEight => "7/8",
        }
    }

    /// How its beats are played to start with: the first accented, and in
    /// compound and odd meters the first of each group too
    pub fn default_beats(self) -> Vec<BeatKind> {
        let (beats, accents): (usize, &[usize]) = match self {
            Meter::TwoFour => (2, &[0]),
            Meter::ThreeFour => (3, &[0]),
            Meter::FourFour => (4, &[0]),
            Meter::FiveFour => (5, &[0]),
            Meter::SixEight => (6, &[0, 3]),
            Meter::SevenEight => (7, &[0, 2, 4]),
        };
        (0..beats)
            .map(|beat| {
                if accents.contains(&beat) {
                    BeatKind::Accent
                } else {
                    BeatKind::Normal
                }
            })
            .collect()
    }
}

/// The practice metronome's settings, as they're shown
#[derive(Clone, PartialEq, Debug)]
pub struct PracticeOptions {
    pub bpm: u32,
    pub meter: Meter,
    pub beats: Vec<BeatKind>,
    pub subdivision: Subdivision,
    pub sound_on: bool,
    pub sound: ClickSound,
    /// From 0 to 1
    pub volume: f64,
    /// Whether the stage flashes on an accented first beat
    pub flash: bool,
    pub trainer_on: bool,
    pub trainer: SpeedTrainer,
    pub silent_bars: bool,
}

impl Default for PracticeOptions {
    fn default() -> Self {
        Self {
            bpm: 100,
            meter: Meter::FourFour,
            beats: Meter::FourFour.default_beats(),
            subdivision: Subdivision::Quarter,
            sound_on: true,
            sound: ClickSound::Click,
            volume: 0.8,
            flash: false,
            trainer_on: false,
            trainer: SpeedTrainer {
                step_bpm: 5.0,
                every_bars: 4,
                up_to_bpm: 140.0,
            },
            silent_bars: false,
        }
    }
}

impl PracticeOptions {
    fn settings(&self, tempo_id: u32) -> PracticeSettings {
        PracticeSettings {
            tempo_id,
            bpm: f64::from(self.bpm),
            beats: self.beats.clone(),
            subdivision: self.subdivision,
            sound: self.sound_on.then_some(self.sound),
            trainer: self.trainer_on.then_some(self.trainer),
            silent_bars: self.silent_bars,
        }
    }
}

/// The standalone metronome, for practising away from the score. It keeps
/// time on the audio thread, and its lights follow the clicks played there
#[derive(Clone, Copy)]
pub struct PracticeMetronome {
    options: Signal<PracticeOptions>,
    /// When the current run started, while it's running
    started_at: Signal<Option<Instant>>,
    /// Time practised in earlier runs
    practised_before: Signal<Duration>,
    /// Whole seconds practised in all, counting the current run
    practised_seconds: Signal<u32>,
    /// The last click played, while running
    click: Signal<Option<ClickPosition>>,
    /// The tempo it's playing at, which the speed trainer raises
    live_bpm: Signal<u32>,
    /// Counts changes of tempo, so the audio thread's reports of the tempo
    /// from before the latest change are ignored rather than undoing it
    tempo_id: Signal<u32>,
}

impl PracticeMetronome {
    pub fn options(&self) -> PracticeOptions {
        (self.options)()
    }

    /// Change the settings, which take effect straight away while running
    pub fn update(&mut self, change: impl FnOnce(&mut PracticeOptions)) {
        let old_bpm = self.options.peek().bpm;
        change(&mut self.options.write());
        let options = self.options.peek();
        audio::set_practice_volume(options.volume as f32);
        if options.bpm != old_bpm {
            self.tempo_id += 1;
        }
        if self.started_at.peek().is_some() {
            audio::update_practice(options.settings(*self.tempo_id.peek()));
            // A new tempo starts the trainer again from it
            if options.bpm != old_bpm {
                self.live_bpm.set(options.bpm);
            }
        }
    }

    pub fn is_running(&self) -> bool {
        (self.started_at)().is_some()
    }

    pub fn start(&mut self) {
        if self.started_at.peek().is_some() {
            return;
        }
        let options = self.options.peek();
        audio::set_practice_volume(options.volume as f32);
        audio::start_practice(options.settings(*self.tempo_id.peek()));
        self.live_bpm.set(options.bpm);
        self.started_at.set(Some(Instant::now()));
    }

    pub fn stop(&mut self) {
        let Some(started_at) = self.started_at.take() else {
            return;
        };
        audio::stop_practice();
        self.practised_before += started_at.elapsed();
        self.click.set(None);
    }

    pub fn toggle(&mut self) {
        if self.started_at.peek().is_some() {
            self.stop();
        } else {
            self.start();
        }
    }

    /// The tempo it's playing at while running, or will start at
    pub fn bpm(&self) -> u32 {
        if self.is_running() {
            (self.live_bpm)()
        } else {
            self.options.read().bpm
        }
    }

    pub fn set_bpm(&mut self, bpm: u32) {
        self.update(|options| options.bpm = bpm.clamp(MIN_BPM, MAX_BPM));
    }

    pub fn set_meter(&mut self, meter: Meter) {
        self.update(|options| {
            options.meter = meter;
            options.beats = meter.default_beats();
        });
    }

    /// Step a beat on from accented to played normally to silent, and round
    pub fn cycle_beat(&mut self, beat: usize) {
        self.update(|options| {
            if let Some(kind) = options.beats.get_mut(beat) {
                *kind = match kind {
                    BeatKind::Accent => BeatKind::Normal,
                    BeatKind::Normal => BeatKind::Silent,
                    BeatKind::Silent => BeatKind::Accent,
                };
            }
        });
    }

    /// Where the last click fell, while running
    pub fn click(&self) -> Option<ClickPosition> {
        (self.click)()
    }

    pub fn practised_seconds(&self) -> u32 {
        (self.practised_seconds)()
    }
}

pub fn use_practice_provider() -> PracticeMetronome {
    let practice = use_context_provider(|| PracticeMetronome {
        options: Signal::new(PracticeOptions::default()),
        started_at: Signal::new(None),
        practised_before: Signal::new(Duration::ZERO),
        practised_seconds: Signal::new(0),
        click: Signal::new(None),
        live_bpm: Signal::new(0),
        tempo_id: Signal::new(0),
    });

    // Bring the lights, tempo and time practised up to date with the audio
    // thread while running
    use_future(move || {
        let mut practice = practice;
        async move {
            loop {
                tokio::time::sleep(POLL_INTERVAL).await;
                let Some(started_at) = *practice.started_at.peek() else {
                    continue;
                };
                let click = audio::click_status();
                if click != *practice.click.peek() {
                    practice.click.set(click);
                }
                let (tempo_id, bpm) = audio::practice_bpm();
                let bpm = bpm.round() as u32;
                if tempo_id == *practice.tempo_id.peek() && bpm != *practice.live_bpm.peek() {
                    practice.live_bpm.set(bpm);
                }
                let seconds = (*practice.practised_before.peek() + started_at.elapsed()).as_secs();
                let seconds = u32::try_from(seconds).unwrap_or(u32::MAX);
                if seconds != *practice.practised_seconds.peek() {
                    practice.practised_seconds.set(seconds);
                }
            }
        }
    });

    practice
}

pub fn use_practice() -> PracticeMetronome {
    use_context::<PracticeMetronome>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compound_and_odd_meters_accent_each_group() {
        use BeatKind::{Accent, Normal};
        assert_eq!(
            Meter::SixEight.default_beats(),
            [Accent, Normal, Normal, Accent, Normal, Normal]
        );
        assert_eq!(
            Meter::SevenEight.default_beats(),
            [Accent, Normal, Accent, Normal, Accent, Normal, Normal]
        );
    }
}

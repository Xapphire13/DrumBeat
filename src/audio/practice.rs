//! The practice metronome, which keeps its own time on the audio thread,
//! apart from the score

use super::click::{ClickKind, ClickPosition, ClickSound, Subdivision};

/// How a beat of the practice metronome's bar is played
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BeatKind {
    Accent,
    Normal,
    /// Not played, though it's still counted
    Silent,
}

/// Every `SILENT_BAR_EVERY`th bar is silent, when silent bars are on
pub const SILENT_BAR_EVERY: u32 = 4;

/// Raises the tempo every few bars, up to a limit
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SpeedTrainer {
    pub step_bpm: f64,
    pub every_bars: u32,
    pub up_to_bpm: f64,
}

#[derive(Clone, PartialEq, Debug)]
pub struct PracticeSettings {
    /// Beats a minute, whatever note the beat is
    pub bpm: f64,
    /// How each beat of the bar is played
    pub beats: Vec<BeatKind>,
    pub subdivision: Subdivision,
    /// `None` with the sound off, for practising by the lights alone
    pub sound: Option<ClickSound>,
    pub trainer: Option<SpeedTrainer>,
    pub silent_bars: bool,
    /// Changes whenever `bpm` is set, so the tempo reported back from before
    /// a change can be told apart from the tempo after it
    pub tempo_id: u32,
}

/// A click the practice metronome plays
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct PracticeClick {
    /// `None` for a click that is counted but not heard
    pub kind: Option<ClickKind>,
    pub position: ClickPosition,
}

pub struct Practice {
    settings: PracticeSettings,
    /// The tempo it's playing at, raised from the settings' by the trainer
    bpm: f64,
    /// Samples until the next click
    until_next: f64,
    /// Where the next click falls, counting bars from 0
    bar: u32,
    beat: u32,
    sub: u32,
}

impl Practice {
    pub fn new(settings: PracticeSettings) -> Self {
        Self {
            bpm: settings.bpm,
            settings,
            until_next: 0.0,
            bar: 0,
            beat: 0,
            sub: 0,
        }
    }

    /// Change the settings without losing the beat. A new tempo starts the
    /// trainer again from it, and the wait for the next click is scaled to
    /// it, so the change is heard straight away
    pub fn update(&mut self, settings: PracticeSettings) {
        if settings.tempo_id != self.settings.tempo_id {
            let new_bpm = settings.bpm.max(1.0);
            self.until_next *= self.bpm.max(1.0) / new_bpm;
            self.bpm = new_bpm;
        }
        self.settings = settings;
        if self.beat >= self.beats() {
            self.beat = 0;
            self.sub = 0;
            self.bar += 1;
        }
    }

    pub fn bpm(&self) -> f64 {
        self.bpm
    }

    pub fn tempo_id(&self) -> u32 {
        self.settings.tempo_id
    }

    pub fn sound(&self) -> Option<ClickSound> {
        self.settings.sound
    }

    fn beats(&self) -> u32 {
        self.settings.beats.len().max(1) as u32
    }

    /// Move on by one sample, returning the click that falls on it, if any
    pub fn tick(&mut self, sample_rate: f64) -> Option<PracticeClick> {
        let click = (self.until_next <= 0.0).then(|| {
            let click = self.click();
            let per_beat = self.settings.subdivision.clicks_per_beat();
            self.until_next += sample_rate * 60.0 / self.bpm.max(1.0) / f64::from(per_beat);
            self.step(per_beat);
            click
        });
        self.until_next -= 1.0;
        click
    }

    fn click(&self) -> PracticeClick {
        let silent = self.settings.silent_bars && (self.bar + 1).is_multiple_of(SILENT_BAR_EVERY);
        let beat_kind = self
            .settings
            .beats
            .get(self.beat as usize)
            .copied()
            .unwrap_or(BeatKind::Normal);
        let kind = match (self.sub, beat_kind) {
            _ if silent || self.settings.sound.is_none() => None,
            (0, BeatKind::Accent) => Some(ClickKind::Accent),
            (0, BeatKind::Normal) => Some(ClickKind::Beat),
            (0, BeatKind::Silent) => None,
            _ => Some(ClickKind::Sub),
        };
        PracticeClick {
            kind,
            position: ClickPosition {
                bar: self.bar + 1,
                beat: self.beat,
                sub: self.sub,
                beats: self.beats(),
                count_in: false,
                silent,
            },
        }
    }

    fn step(&mut self, per_beat: u32) {
        self.sub += 1;
        if self.sub < per_beat {
            return;
        }
        self.sub = 0;
        self.beat += 1;
        if self.beat < self.beats() {
            return;
        }
        self.beat = 0;
        self.bar += 1;
        if let Some(trainer) = self.settings.trainer
            && trainer.every_bars > 0
            && self.bar.is_multiple_of(trainer.every_bars)
            && self.bpm < trainer.up_to_bpm
        {
            self.bpm = (self.bpm + trainer.step_bpm).min(trainer.up_to_bpm);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> PracticeSettings {
        PracticeSettings {
            bpm: 60.0,
            beats: vec![BeatKind::Accent, BeatKind::Normal, BeatKind::Silent],
            subdivision: Subdivision::Quarter,
            sound: Some(ClickSound::Click),
            trainer: None,
            silent_bars: false,
            tempo_id: 0,
        }
    }

    /// Run for `samples` at 100 samples a second, returning each click and
    /// the sample it fell on
    fn run(practice: &mut Practice, samples: usize) -> Vec<(usize, PracticeClick)> {
        (0..samples)
            .filter_map(|i| practice.tick(100.0).map(|click| (i, click)))
            .collect()
    }

    #[test]
    fn clicks_on_the_beat() {
        let mut practice = Practice::new(settings());
        let clicks = run(&mut practice, 400);
        let samples: Vec<_> = clicks.iter().map(|(i, _)| *i).collect();
        assert_eq!(samples, [0, 100, 200, 300]);
        let kinds: Vec<_> = clicks.iter().map(|(_, c)| c.kind).collect();
        assert_eq!(
            kinds,
            [
                Some(ClickKind::Accent),
                Some(ClickKind::Beat),
                None,
                Some(ClickKind::Accent)
            ]
        );
        assert_eq!(clicks[3].1.position.bar, 2);
    }

    #[test]
    fn silences_every_fourth_bar() {
        let mut practice = Practice::new(PracticeSettings {
            silent_bars: true,
            ..settings()
        });
        let clicks = run(&mut practice, 1500);
        let silent: Vec<_> = clicks
            .iter()
            .map(|(_, c)| (c.position.bar, c.position.silent, c.kind.is_some()))
            .filter(|&(bar, ..)| bar == 4)
            .collect();
        assert_eq!(silent, [(4, true, false); 3]);
    }

    #[test]
    fn trainer_raises_the_tempo_up_to_its_limit() {
        let mut practice = Practice::new(PracticeSettings {
            trainer: Some(SpeedTrainer {
                step_bpm: 30.0,
                every_bars: 1,
                up_to_bpm: 100.0,
            }),
            ..settings()
        });
        // A bar at 60, then at 90
        run(&mut practice, 300);
        assert_eq!(practice.bpm(), 90.0);
        run(&mut practice, 200);
        assert_eq!(practice.bpm(), 100.0);
        run(&mut practice, 600);
        assert_eq!(practice.bpm(), 100.0);
    }

    #[test]
    fn a_new_tempo_is_heard_straight_away() {
        let mut practice = Practice::new(settings());
        // Halfway to the second beat at 60 BPM, double the tempo
        run(&mut practice, 50);
        practice.update(PracticeSettings {
            bpm: 120.0,
            tempo_id: 1,
            ..settings()
        });
        let clicks = run(&mut practice, 100);
        let samples: Vec<_> = clicks.iter().map(|(i, _)| *i).collect();
        assert_eq!(samples, [25, 75]);
    }

    #[test]
    fn subdivides_the_beat() {
        let mut practice = Practice::new(PracticeSettings {
            subdivision: Subdivision::Sixteenth,
            ..settings()
        });
        let clicks = run(&mut practice, 100);
        let subs: Vec<_> = clicks.iter().map(|(_, c)| c.position.sub).collect();
        assert_eq!(subs, [0, 1, 2, 3]);
    }
}

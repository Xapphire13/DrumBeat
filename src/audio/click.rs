//! The metronome: its clicks over the score while it plays, the count-in
//! before it, and where the last click fell, which the UI lights its beats
//! from so they stay in step with what is heard

use std::sync::atomic::{AtomicU64, Ordering};

use crate::data::{note::TICKS_PER_QUARTER, score::Score};

/// The sound the metronome clicks with
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ClickSound {
    #[default]
    Click,
    Wood,
    Rim,
    Cowbell,
}

impl ClickSound {
    pub fn label(self) -> &'static str {
        match self {
            ClickSound::Click => "Click",
            ClickSound::Wood => "Wood",
            ClickSound::Rim => "Rim",
            ClickSound::Cowbell => "Cowbell",
        }
    }
}

/// How many clicks there are to a beat
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Subdivision {
    #[default]
    Quarter,
    Eighth,
    EighthTriplet,
    Sixteenth,
}

impl Subdivision {
    pub const ALL: [Subdivision; 4] = [
        Subdivision::Quarter,
        Subdivision::Eighth,
        Subdivision::EighthTriplet,
        Subdivision::Sixteenth,
    ];

    pub fn clicks_per_beat(self) -> u32 {
        match self {
            Subdivision::Quarter => 1,
            Subdivision::Eighth => 2,
            Subdivision::EighthTriplet => 3,
            Subdivision::Sixteenth => 4,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Subdivision::Quarter => "1/4",
            Subdivision::Eighth => "1/8",
            Subdivision::EighthTriplet => "1/8T",
            Subdivision::Sixteenth => "1/16",
        }
    }
}

/// How the metronome plays along with the score
#[derive(Clone, PartialEq, Debug)]
pub struct MetronomeSettings {
    /// Whether it clicks along with the score. The count-in plays either way
    pub enabled: bool,
    pub sound: ClickSound,
    /// From 0 to 1, apart from the music's volume
    pub volume: f64,
    pub subdivision: Subdivision,
    /// Bars clicked before the score starts playing
    pub count_in_bars: u32,
    /// Whether the first beat of each measure is accented
    pub accent_first: bool,
}

impl Default for MetronomeSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            sound: ClickSound::Click,
            volume: 0.8,
            subdivision: Subdivision::Quarter,
            count_in_bars: 0,
            accent_first: true,
        }
    }
}

/// How strongly a click is played
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ClickKind {
    /// The first beat of a measure, played higher
    Accent,
    Beat,
    /// A click between beats, played quieter
    Sub,
}

impl ClickKind {
    pub fn velocity(self) -> f32 {
        match self {
            ClickKind::Accent => 1.0,
            ClickKind::Beat => 0.75,
            ClickKind::Sub => 0.45,
        }
    }
}

/// Where a click falls in its bar
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ClickPosition {
    /// Counting from 1
    pub bar: u32,
    /// Counting from 0
    pub beat: u32,
    /// The click within the beat, counting from 0 on the beat itself
    pub sub: u32,
    /// Beats in the bar
    pub beats: u32,
    /// Whether the click is counting in, before the score plays
    pub count_in: bool,
    /// Whether the bar is left silent, for keeping time without the click
    pub silent: bool,
}

/// A metronome click on the performance timeline
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Click {
    /// Seconds into the performance at the score's own tempo
    pub seconds: f64,
    pub kind: ClickKind,
    pub position: ClickPosition,
}

/// Every click as the score is played, with repeats expanded, sorted by
/// time. Each played measure is clicked through by the beats of its time
/// signature, and bars are counted as they're played
pub fn score_clicks(score: &Score, settings: &MetronomeSettings) -> Vec<Click> {
    let performance = &score.performance;
    let tempo_map = performance.tempo_map();
    let per_beat = settings.subdivision.clicks_per_beat();
    let mut clicks = vec![];

    for (index, played) in performance.measures().iter().enumerate() {
        let time_signature = &score.measures[played.measure_index].time_signature;
        let beat_ticks = time_signature.note_value.get_ticks();
        let beats = time_signature.notes_per_measure;
        for beat in 0.. {
            let beat_start = beat * beat_ticks;
            if beat_start >= played.length {
                break;
            }
            for sub in 0..per_beat {
                let offset = beat_start + beat_ticks * sub / per_beat;
                if offset >= played.length {
                    break;
                }
                clicks.push(Click {
                    seconds: tempo_map.ticks_to_seconds(played.timestamp + offset),
                    kind: kind(beat, sub, settings.accent_first),
                    position: ClickPosition {
                        bar: index as u32 + 1,
                        // A measure lengthened past its time signature keeps
                        // its last beat lit
                        beat: beat.min(beats.saturating_sub(1)),
                        sub,
                        beats,
                        count_in: false,
                        silent: false,
                    },
                });
            }
        }
    }

    clicks
}

/// The clicks counting in to playback from `timestamp` on the performance
/// timeline, in whole bars of the time signature and tempo there. They're
/// timed in seconds back from where playback starts, so they run from minus
/// the count-in's length to 0. Bars are counted from 1
pub fn count_in_clicks(score: &Score, timestamp: u32, settings: &MetronomeSettings) -> Vec<Click> {
    let performance = &score.performance;
    let Some(played) = performance
        .measures()
        .get(performance.played_measure_index(timestamp))
    else {
        return vec![];
    };
    let time_signature = &score.measures[played.measure_index].time_signature;
    let beats = time_signature.notes_per_measure;
    let bpm = performance.tempo_map().bpm_at(timestamp);
    let beat_seconds = f64::from(time_signature.note_value.get_ticks()) * 60.0
        / (bpm * f64::from(TICKS_PER_QUARTER));
    let total = settings.count_in_bars * beats;

    (0..total)
        .map(|i| {
            let beat = i % beats;
            Click {
                seconds: -f64::from(total - i) * beat_seconds,
                kind: kind(beat, 0, settings.accent_first),
                position: ClickPosition {
                    bar: i / beats + 1,
                    beat,
                    sub: 0,
                    beats,
                    count_in: true,
                    silent: false,
                },
            }
        })
        .collect()
}

fn kind(beat: u32, sub: u32, accent_first: bool) -> ClickKind {
    match (beat, sub) {
        (0, 0) if accent_first => ClickKind::Accent,
        (_, 0) => ClickKind::Beat,
        _ => ClickKind::Sub,
    }
}

/// Where the last click fell, packed for the audio thread to set without
/// locking. Zero while the metronome isn't clicking
static STATUS: AtomicU64 = AtomicU64::new(0);

const ACTIVE: u64 = 1 << 63;
const COUNT_IN: u64 = 1 << 62;
const SILENT: u64 = 1 << 61;

/// Set by the audio thread as each click is played, or `None` when the
/// metronome stops
pub(super) fn publish(position: Option<ClickPosition>) {
    STATUS.store(position.map_or(0, pack), Ordering::Relaxed);
}

/// Where the click the audio thread last played fell, while the metronome
/// is going
pub fn status() -> Option<ClickPosition> {
    unpack(STATUS.load(Ordering::Relaxed))
}

fn pack(position: ClickPosition) -> u64 {
    let byte = |value: u32| u64::from(value.min(0xff));
    ACTIVE
        | if position.count_in { COUNT_IN } else { 0 }
        | if position.silent { SILENT } else { 0 }
        | u64::from(position.bar.min(0xff_ffff)) << 24
        | byte(position.beat) << 16
        | byte(position.sub) << 8
        | byte(position.beats)
}

fn unpack(bits: u64) -> Option<ClickPosition> {
    if bits & ACTIVE == 0 {
        return None;
    }
    let byte = |shift: u32| (bits >> shift & 0xff) as u32;
    Some(ClickPosition {
        bar: (bits >> 24 & 0xff_ffff) as u32,
        beat: byte(16),
        sub: byte(8),
        beats: byte(0),
        count_in: bits & COUNT_IN != 0,
        silent: bits & SILENT != 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{
        measure::{Fill, Measure},
        note::NoteValue,
        tempo::TempoMap,
        time_signature::TimeSignature,
    };

    fn score(signatures: &[(u32, NoteValue)], repeat: bool) -> Score {
        let mut timestamp = 0;
        let measures = signatures
            .iter()
            .enumerate()
            .map(|(i, &(notes_per_measure, note_value))| {
                let measure = Measure {
                    timestamp,
                    notes: vec![],
                    rests: vec![],
                    grace_notes: vec![],
                    time_signature: TimeSignature {
                        notes_per_measure,
                        note_value,
                    },
                    repeat_start: false,
                    section: None,
                    repeat_end: (repeat && i == 0).then_some(2),
                    fill: Fill::Full,
                };
                timestamp += measure.written_length();
                measure
            })
            .collect();
        Score::new(String::new(), measures, TempoMap::new(120.0))
    }

    #[test]
    fn clicks_each_beat_of_each_played_measure() {
        let score = score(&[(4, NoteValue::Quarter), (3, NoteValue::Quarter)], true);
        let clicks = score_clicks(&score, &MetronomeSettings::default());
        // The first measure is played twice, then the 3/4 one
        assert_eq!(clicks.len(), 4 + 4 + 3);
        let bars: Vec<_> = clicks.iter().map(|c| c.position.bar).collect();
        assert_eq!(bars, [1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3]);
        assert_eq!(clicks[0].kind, ClickKind::Accent);
        assert_eq!(clicks[1].kind, ClickKind::Beat);
        assert_eq!(clicks[8].position.beats, 3);
        // Half a second a beat at 120 BPM
        assert!((clicks[9].seconds - 4.5).abs() < 1e-9);
    }

    #[test]
    fn subdivides_beats() {
        let score = score(&[(2, NoteValue::Quarter)], false);
        let settings = MetronomeSettings {
            subdivision: Subdivision::EighthTriplet,
            accent_first: false,
            ..MetronomeSettings::default()
        };
        let clicks = score_clicks(&score, &settings);
        let kinds: Vec<_> = clicks.iter().map(|c| c.kind).collect();
        use ClickKind::{Beat, Sub};
        assert_eq!(kinds, [Beat, Sub, Sub, Beat, Sub, Sub]);
        assert!((clicks[1].seconds - 0.5 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn counts_in_whole_bars_before_the_start() {
        let score = score(&[(4, NoteValue::Quarter), (6, NoteValue::Eighth)], false);
        let settings = MetronomeSettings {
            count_in_bars: 2,
            ..MetronomeSettings::default()
        };
        // From the 6/8 measure: eighths at 120 BPM are a quarter second
        let clicks = count_in_clicks(&score, 3840, &settings);
        assert_eq!(clicks.len(), 12);
        assert!((clicks[0].seconds + 3.0).abs() < 1e-9);
        assert!((clicks[11].seconds + 0.25).abs() < 1e-9);
        assert_eq!(clicks[6].position.bar, 2);
        assert_eq!(clicks[6].kind, ClickKind::Accent);
        assert!(clicks.iter().all(|c| c.position.count_in));
    }

    #[test]
    fn status_round_trips() {
        let position = ClickPosition {
            bar: 1234,
            beat: 6,
            sub: 2,
            beats: 7,
            count_in: true,
            silent: true,
        };
        assert_eq!(unpack(pack(position)), Some(position));
        assert_eq!(unpack(0), None);
    }
}

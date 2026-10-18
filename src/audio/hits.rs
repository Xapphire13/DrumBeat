use super::voice::Sound;
use crate::data::score::Score;

/// How loud grace notes are compared with main notes
const GRACE_NOTE_VELOCITY: f32 = 0.45;

/// A drum hit on the performance timeline
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Hit {
    /// When the hit is played, in seconds into the performance at the
    /// score's own tempo
    pub seconds: f64,
    pub sound: Sound,
    /// The General MIDI note it's written as, for MIDI out
    pub midi_note: u8,
    /// From 0 to 1
    pub velocity: f32,
}

/// Every hit in the score as it is played, with repeats expanded, sorted by
/// time
pub fn score_hits(score: &Score) -> Vec<Hit> {
    let performance = &score.performance;
    let tempo_map = performance.tempo_map();
    let mut hits = vec![];

    for played in performance.measures() {
        let measure = &score.measures[played.measure_index];
        let to_performance = |written: u32| played.timestamp + written - played.written_timestamp;

        for note in measure.notes.iter().filter(|note| !note.tied) {
            if let Some(piece) = note.drum_piece() {
                hits.push(Hit {
                    seconds: tempo_map.ticks_to_seconds(to_performance(note.timestamp)),
                    sound: Sound::new(piece, note.articulation),
                    midi_note: note.midi_note,
                    velocity: 1.0,
                });
            }
        }

        for grace_note in &measure.grace_notes {
            if let Some(piece) = grace_note.drum_piece() {
                // A grace note can fall just before its measure, so place it
                // relative to its main note
                let lead = grace_note.main_timestamp - grace_note.timestamp;
                let timestamp = to_performance(grace_note.main_timestamp).saturating_sub(lead);
                hits.push(Hit {
                    seconds: tempo_map.ticks_to_seconds(timestamp),
                    sound: Sound::new(piece, grace_note.articulation),
                    midi_note: grace_note.midi_note,
                    velocity: GRACE_NOTE_VELOCITY,
                });
            }
        }
    }

    hits.sort_by(|a, b| a.seconds.total_cmp(&b.seconds));
    hits
}

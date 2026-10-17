//! Conversion from the loader's types into the app's data model

use crate::data::{
    measure::Measure,
    note::{Duration, Note, NoteValue, TICKS_PER_QUARTER, Tuplet as DataTuplet},
    score::Score,
    tempo::{TempoChange as DataTempoChange, TempoMap},
    time_signature::TimeSignature as DataTimeSignature,
};

use super::{Fraction, GpError, GpNoteValue, GpRhythm, GpScore, TimeSignature};

const TICKS_PER_WHOLE: u32 = 4 * TICKS_PER_QUARTER;
const DEFAULT_BPM: f64 = 120.0;

impl GpScore {
    /// Convert into the app's score. Repeats are not expanded, so the score
    /// plays straight through as written. Grace notes are dropped
    pub fn to_score(&self) -> Result<Score, GpError> {
        let mut measures = Vec::with_capacity(self.bars.len());
        let mut bar_timestamp = 0;

        for bar in &self.bars {
            let notes = bar
                .voices
                .iter()
                .flat_map(|voice| &voice.beats)
                .filter(|beat| beat.grace.is_none())
                .flat_map(|beat| {
                    let timestamp = bar_timestamp + to_ticks(beat.start);
                    let duration = convert_rhythm(&beat.rhythm);
                    beat.notes.iter().map(move |note| Note {
                        duration,
                        midi_note: note.midi_note,
                        timestamp,
                        tied: note.tie_destination,
                    })
                })
                .collect();

            let measure = Measure {
                timestamp: bar_timestamp,
                notes,
                time_signature: convert_time_signature(&bar.time_signature)?,
            };
            bar_timestamp += measure.get_length();
            measures.push(measure);
        }

        let mut tempo_map = TempoMap::new(DEFAULT_BPM);
        for change in &self.tempo_changes {
            let Some(measure) = measures.get(change.bar) else {
                continue;
            };
            let offset = (change.position * f64::from(measure.get_length())).round() as u32;
            tempo_map.add_change(DataTempoChange {
                timestamp: measure.timestamp + offset,
                bpm: change.bpm,
            });
        }

        Ok(Score {
            title: self.title.clone(),
            measures,
            tempo_map,
        })
    }
}

/// Convert an offset in whole notes to ticks, rounding when it doesn't land
/// on a whole tick
fn to_ticks(fraction: Fraction) -> u32 {
    fraction
        .to_ticks(TICKS_PER_WHOLE)
        .unwrap_or_else(|| (fraction.as_f64() * f64::from(TICKS_PER_WHOLE)).round() as u32)
}

fn convert_rhythm(rhythm: &GpRhythm) -> Duration {
    let value = match rhythm.value {
        GpNoteValue::Whole => NoteValue::Whole,
        GpNoteValue::Half => NoteValue::Half,
        GpNoteValue::Quarter => NoteValue::Quarter,
        GpNoteValue::Eighth => NoteValue::Eighth,
        GpNoteValue::Sixteenth => NoteValue::Sixteenth,
        GpNoteValue::ThirtySecond => NoteValue::ThirtySecond,
        // The app has no 128th notes; only the drawn value is affected
        GpNoteValue::SixtyFourth | GpNoteValue::OneTwentyEighth => NoteValue::SixtyFourth,
    };

    Duration {
        dots: rhythm.dots,
        tuplet: rhythm.tuplet.map(|t| DataTuplet {
            notes: t.notes,
            in_time_of: t.in_time_of,
        }),
        ..Duration::new(value)
    }
}

fn convert_time_signature(time_signature: &TimeSignature) -> Result<DataTimeSignature, GpError> {
    let note_value = match time_signature.note_value {
        1 => NoteValue::Whole,
        2 => NoteValue::Half,
        4 => NoteValue::Quarter,
        8 => NoteValue::Eighth,
        16 => NoteValue::Sixteenth,
        32 => NoteValue::ThirtySecond,
        64 => NoteValue::SixtyFourth,
        other => {
            return Err(GpError::Invalid(format!(
                "unsupported time signature {}/{other}",
                time_signature.notes_per_measure
            )));
        }
    };

    Ok(DataTimeSignature {
        notes_per_measure: time_signature.notes_per_measure,
        note_value,
    })
}

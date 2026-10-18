//! Conversion from the loader's types into the app's data model

use crate::data::{
    drum_piece::DrumPiece,
    measure::Measure,
    note::{
        Annotation, Articulation, Duration, Note, NoteValue, Rest, TICKS_PER_QUARTER,
        Tuplet as DataTuplet,
    },
    score::{Hairpin as DataHairpin, HairpinKind, Score},
    tempo::{TempoChange as DataTempoChange, TempoMap},
    time_signature::TimeSignature as DataTimeSignature,
};

use super::{Fraction, GpError, GpNoteValue, GpRhythm, GpScore, Hairpin, TimeSignature};

const TICKS_PER_WHOLE: u32 = 4 * TICKS_PER_QUARTER;
const DEFAULT_BPM: f64 = 120.0;

impl GpScore {
    /// Convert into the app's score. Grace notes are dropped
    pub fn to_score(&self) -> Result<Score, GpError> {
        let mut measures = Vec::with_capacity(self.bars.len());
        let mut bar_timestamp = 0;

        for bar in &self.bars {
            let mut notes = vec![];
            let mut rests = vec![];
            for (voice, index) in bar.voices.iter().zip(0..) {
                for beat in voice.beats.iter().filter(|beat| beat.grace.is_none()) {
                    let timestamp = bar_timestamp + to_ticks(beat.start);
                    let duration = convert_rhythm(&beat.rhythm);
                    if beat.notes.is_empty() {
                        rests.push(Rest {
                            duration,
                            timestamp,
                            voice: index,
                        });
                    }
                    notes.extend(beat.notes.iter().map(|note| Note {
                        duration,
                        midi_note: note.midi_note,
                        timestamp,
                        tied: note.tie_destination,
                        voice: index,
                        articulation: Articulation::new(
                            note.midi_note,
                            note.articulation.as_deref(),
                        ),
                        annotation: None,
                    }));
                }
            }

            let measure = Measure {
                timestamp: bar_timestamp,
                notes,
                rests,
                time_signature: convert_time_signature(&bar.time_signature)?,
                repeat_start: bar.repeat_start,
                repeat_end: bar.repeat_end,
            };
            bar_timestamp += measure.get_length();
            measures.push(measure);
        }
        annotate_hihats(&mut measures);
        let hairpins = self.hairpins(&measures);

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

        let mut score = Score::new(self.title.clone(), measures, tempo_map);
        score.hairpins = hairpins;
        Ok(score)
    }

    /// Join runs of beats under the same kind of hairpin, in any voice, into
    /// hairpins on the score's timeline
    fn hairpins(&self, measures: &[Measure]) -> Vec<DataHairpin> {
        let mut beats: Vec<(u32, u32, HairpinKind)> = vec![];
        for (bar, measure) in self.bars.iter().zip(measures) {
            for beat in bar.voices.iter().flat_map(|voice| &voice.beats) {
                let Some(hairpin) = beat.hairpin else {
                    continue;
                };
                let start = measure.timestamp + to_ticks(beat.start);
                let kind = match hairpin {
                    Hairpin::Crescendo => HairpinKind::Crescendo,
                    Hairpin::Decrescendo => HairpinKind::Decrescendo,
                };
                beats.push((start, start + to_ticks(beat.rhythm.duration()), kind));
            }
        }
        beats.sort_by_key(|&(start, ..)| start);

        let mut hairpins: Vec<DataHairpin> = vec![];
        for (start, end, kind) in beats {
            match hairpins.last_mut() {
                Some(last) if last.kind == kind && start <= last.end => {
                    last.end = last.end.max(end);
                }
                _ => hairpins.push(DataHairpin { kind, start, end }),
            }
        }
        hairpins
    }
}

/// Mark open and half-open hi-hats, and closed ones that follow an open one
/// (where the pedal closes the hi-hat again)
fn annotate_hihats(measures: &mut [Measure]) {
    let mut hihats: Vec<&mut Note> = measures
        .iter_mut()
        .flat_map(|measure| &mut measure.notes)
        .filter(|note| {
            matches!(
                note.drum_piece(),
                Some(DrumPiece::Hihat | DrumPiece::HihatPedal)
            )
        })
        .collect();
    hihats.sort_by_key(|note| note.timestamp);

    let mut is_open = false;
    for note in hihats {
        let is_pedal = note.drum_piece() == Some(DrumPiece::HihatPedal);
        note.annotation = match note.articulation {
            Articulation::HihatOpen => Some(Annotation::Open),
            Articulation::HihatHalfOpen => Some(Annotation::HalfOpen),
            _ if is_open && !is_pedal => Some(Annotation::Closed),
            _ => None,
        };
        is_open = note.articulation.is_open_hihat();
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
        // Guitar Pro stores some tuplets, e.g. sextuplets, only as the
        // secondary one. The app keeps just one, the outermost
        tuplet: rhythm.tuplet.or(rhythm.inner_tuplet).map(|t| DataTuplet {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_hihat_opening_and_closing() {
        let hihat = |timestamp, midi_note, name| Note {
            duration: Duration::new(NoteValue::Eighth),
            midi_note,
            timestamp,
            tied: false,
            voice: 0,
            articulation: Articulation::new(midi_note, Some(name)),
            annotation: None,
        };
        let mut measures = vec![Measure {
            timestamp: 0,
            notes: vec![
                hihat(0, 42, "Hi-Hat (closed)"),
                hihat(480, 46, "Hi-Hat (open)"),
                hihat(960, 42, "Hi-Hat (closed)"),
                hihat(1440, 42, "Hi-Hat (closed)"),
                hihat(1920, 46, "Hi-Hat (half)"),
                // The pedal closes it, so the next closed hit needs no mark
                hihat(2400, 44, "Pedal Hi-Hat (hit)"),
                hihat(2880, 42, "Hi-Hat (closed)"),
            ],
            rests: vec![],
            time_signature: DataTimeSignature {
                notes_per_measure: 4,
                note_value: NoteValue::Quarter,
            },
            repeat_start: false,
            repeat_end: None,
        }];
        annotate_hihats(&mut measures);

        let annotations: Vec<_> = measures[0].notes.iter().map(|n| n.annotation).collect();
        assert_eq!(
            annotations,
            vec![
                None,
                Some(Annotation::Open),
                Some(Annotation::Closed),
                None,
                Some(Annotation::HalfOpen),
                None,
                None,
            ]
        );
        assert_eq!(
            Articulation::new(53, Some("Ride (bell)")),
            Articulation::RideBell
        );
    }
}

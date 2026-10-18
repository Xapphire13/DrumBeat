use super::{measure::Measure, performance::Performance};

/// A stretch of the performance that plays over and over, from one note to
/// the end of another. Timestamps are on the performance timeline, so a loop
/// can be one particular pass through a repeat
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LoopRange {
    /// Where the first note is played
    pub start: u32,
    /// Where the last note is played
    pub last: u32,
    /// Where playback goes back to `start`: the next note after `last`, or
    /// the end of its measure
    pub end: u32,
}

impl LoopRange {
    /// A loop from the note at `a` to the note at `b`, in either order. Both
    /// should be snap points of the performance
    pub fn between_notes(a: u32, b: u32, performance: &Performance, measures: &[Measure]) -> Self {
        let (start, last) = (a.min(b), a.max(b));
        let end = performance
            .snap_points(measures)
            .into_iter()
            .find(|&timestamp| timestamp > last)
            .unwrap_or_else(|| performance.end_timestamp())
            .max(last + 1);
        Self { start, last, end }
    }

    /// A loop over whole played measures, from the one at index `a` in
    /// `performance.measures()` to the one at `b`, in either order
    pub fn between_measures(
        a: usize,
        b: usize,
        performance: &Performance,
        measures: &[Measure],
    ) -> Option<Self> {
        let played = performance.measures();
        let (first, last) = (played.get(a.min(b))?, played.get(a.max(b))?);
        let (start, end) = (first.timestamp, last.timestamp + last.length);
        let last = performance
            .snap_points(measures)
            .into_iter()
            .take_while(|&timestamp| timestamp < end)
            .last()
            .unwrap_or(start)
            .max(start);
        Some(Self { start, last, end })
    }

    pub fn contains(&self, timestamp: u32) -> bool {
        self.start <= timestamp && timestamp < self.end
    }

    /// How long the loop is, in beats of each measure's time signature
    pub fn beats(&self, performance: &Performance, measures: &[Measure]) -> f64 {
        performance
            .measures()
            .iter()
            .map(|played| {
                let from = self.start.max(played.timestamp);
                let to = self.end.min(played.timestamp + played.length);
                let beat = measures[played.measure_index]
                    .time_signature
                    .note_value
                    .get_ticks();
                f64::from(to.saturating_sub(from)) / f64::from(beat)
            })
            .sum()
    }

    /// The part of the played measure at `index` that the loop covers, as
    /// fractions of the measure from its start, if it covers any of it
    pub fn measure_coverage(&self, performance: &Performance, index: usize) -> Option<(f64, f64)> {
        let played = performance.measures().get(index)?;
        let measure_end = played.timestamp + played.length;
        if self.start >= measure_end || self.end <= played.timestamp || played.length == 0 {
            return None;
        }
        let fraction = |timestamp: u32| {
            f64::from(timestamp.clamp(played.timestamp, measure_end) - played.timestamp)
                / f64::from(played.length)
        };
        Some((fraction(self.start), fraction(self.end)))
    }

    /// The stretches of the written score the loop plays, in order, with
    /// stretches that follow on from each other joined up
    pub fn written_spans(&self, performance: &Performance) -> Vec<(u32, u32)> {
        let mut spans: Vec<(u32, u32)> = vec![];
        for played in performance.measures() {
            let from = self.start.max(played.timestamp);
            let to = self.end.min(played.timestamp + played.length);
            if from >= to {
                continue;
            }
            let span = (
                played.written_timestamp + from - played.timestamp,
                played.written_timestamp + to - played.timestamp,
            );
            match spans.last_mut() {
                Some((_, end)) if *end == span.0 => *end = span.1,
                _ => spans.push(span),
            }
        }
        spans
    }
}

/// Where a performance timestamp falls, counted as a drummer would: the
/// played measure's number, the beat, and how far into the beat, e.g. "17:2"
/// for measure 17 beat 2, or "20:4&" for the and of beat 4
pub fn count_label(timestamp: u32, performance: &Performance, measures: &[Measure]) -> String {
    let index = performance.played_measure_index(timestamp);
    let Some(played) = performance.measures().get(index) else {
        return String::new();
    };
    let beat = measures[played.measure_index]
        .time_signature
        .note_value
        .get_ticks();
    let offset = timestamp.saturating_sub(played.timestamp);
    let number = index + 1;
    let beat_number = offset / beat + 1;
    let subdivision = subdivision_label(offset % beat, beat);
    format!("{number}:{beat_number}{subdivision}")
}

/// How far into a beat `ticks` is: "e", "&" and "a" for sixteenths, a
/// fraction for anything else, and nothing on the beat
fn subdivision_label(ticks: u32, beat: u32) -> String {
    if ticks == 0 {
        return String::new();
    }
    let divisor = gcd(ticks, beat);
    match (ticks / divisor, beat / divisor) {
        (1, 2) => "&".into(),
        (1, 4) => "e".into(),
        (3, 4) => "a".into(),
        (1, 3) => "⅓".into(),
        (2, 3) => "⅔".into(),
        (1, 6) => "⅙".into(),
        (5, 6) => "⅚".into(),
        (1, 8) => "⅛".into(),
        (3, 8) => "⅜".into(),
        (5, 8) => "⅝".into(),
        (7, 8) => "⅞".into(),
        (numerator, denominator) => format!("+{numerator}/{denominator}"),
    }
}

fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 { a } else { gcd(b, a % b) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{
        measure::Fill,
        note::{Articulation, Duration, Note, NoteValue},
        tempo::TempoMap,
        time_signature::TimeSignature,
    };

    const MEASURE: u32 = 3840;
    const EIGHTH: u32 = 480;

    /// 4/4 measures of eighth notes, `(repeat_start, repeat_end)` for each
    fn measures(repeats: &[(bool, Option<u32>)]) -> Vec<Measure> {
        repeats
            .iter()
            .enumerate()
            .map(|(i, &(repeat_start, repeat_end))| {
                let timestamp = i as u32 * MEASURE;
                Measure {
                    timestamp,
                    notes: (0..8)
                        .map(|n| Note {
                            duration: Duration::new(NoteValue::Eighth),
                            midi_note: 42,
                            timestamp: timestamp + n * EIGHTH,
                            tied: false,
                            voice: 0,
                            articulation: Articulation::Normal,
                            annotation: None,
                        })
                        .collect(),
                    time_signature: TimeSignature {
                        notes_per_measure: 4,
                        note_value: NoteValue::Quarter,
                    },
                    repeat_start,
                    rests: vec![],
                    grace_notes: vec![],
                    repeat_end,
                    fill: Fill::Full,
                }
            })
            .collect()
    }

    fn performance(measures: &[Measure]) -> Performance {
        Performance::new(measures, &TempoMap::new(120.0))
    }

    #[test]
    fn note_loop_runs_to_the_note_after_the_last() {
        let measures = measures(&[(false, None); 4]);
        let performance = performance(&measures);
        // Dragged backwards, from beat 4& of measure 3 to beat 2 of measure 1
        let range = LoopRange::between_notes(
            2 * MEASURE + 7 * EIGHTH,
            2 * EIGHTH,
            &performance,
            &measures,
        );
        assert_eq!(range.start, 2 * EIGHTH);
        assert_eq!(range.last, 2 * MEASURE + 7 * EIGHTH);
        assert_eq!(range.end, 3 * MEASURE);
        assert!(range.contains(range.start));
        assert!(!range.contains(range.end));
        assert_eq!(count_label(range.start, &performance, &measures), "1:2");
        assert_eq!(count_label(range.last, &performance, &measures), "3:4&");
        assert!((range.beats(&performance, &measures) - 11.0).abs() < 1e-9);
    }

    #[test]
    fn loop_on_the_last_note_ends_with_the_score() {
        let measures = measures(&[(false, None); 2]);
        let performance = performance(&measures);
        let last = 2 * MEASURE - EIGHTH;
        let range = LoopRange::between_notes(last, last, &performance, &measures);
        assert_eq!(range.end, 2 * MEASURE);
        assert!((range.beats(&performance, &measures) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn measure_loop_covers_whole_measures() {
        let measures = measures(&[(false, None); 4]);
        let performance = performance(&measures);
        let range = LoopRange::between_measures(2, 1, &performance, &measures).unwrap();
        assert_eq!(range.start, MEASURE);
        assert_eq!(range.last, 3 * MEASURE - EIGHTH);
        assert_eq!(range.end, 3 * MEASURE);
        assert_eq!(
            LoopRange::between_measures(1, 9, &performance, &measures),
            None
        );
    }

    #[test]
    fn coverage_is_partial_where_the_loop_starts_and_ends_mid_measure() {
        let measures = measures(&[(false, None); 4]);
        let performance = performance(&measures);
        let range = LoopRange::between_notes(
            2 * EIGHTH,
            2 * MEASURE + 3 * EIGHTH,
            &performance,
            &measures,
        );
        assert_eq!(range.measure_coverage(&performance, 0), Some((0.25, 1.0)));
        assert_eq!(range.measure_coverage(&performance, 1), Some((0.0, 1.0)));
        assert_eq!(range.measure_coverage(&performance, 2), Some((0.0, 0.5)));
        assert_eq!(range.measure_coverage(&performance, 3), None);
    }

    #[test]
    fn loop_in_a_repeat_is_one_pass() {
        // Measure 0, then 1 played twice, then 2
        let measures = measures(&[(false, None), (true, Some(2)), (false, None)]);
        let performance = performance(&measures);
        // The second time through measure 1, which is played as measure 3
        let range = LoopRange::between_measures(2, 2, &performance, &measures).unwrap();
        assert_eq!(range.start, 2 * MEASURE);
        assert_eq!(count_label(range.start, &performance, &measures), "3:1");
        assert_eq!(
            range.written_spans(&performance),
            vec![(MEASURE, 2 * MEASURE)]
        );
        assert_eq!(range.measure_coverage(&performance, 1), None);
    }

    #[test]
    fn written_spans_split_where_playback_jumps_back() {
        let measures = measures(&[(false, None), (true, Some(2)), (false, None)]);
        let performance = performance(&measures);
        // From halfway through the first pass of measure 1 into the second
        let range = LoopRange::between_notes(
            MEASURE + 4 * EIGHTH,
            2 * MEASURE + 3 * EIGHTH,
            &performance,
            &measures,
        );
        assert_eq!(
            range.written_spans(&performance),
            vec![
                (MEASURE + 4 * EIGHTH, 2 * MEASURE),
                (MEASURE, MEASURE + 4 * EIGHTH)
            ]
        );
    }

    #[test]
    fn subdivisions() {
        assert_eq!(subdivision_label(0, 960), "");
        assert_eq!(subdivision_label(240, 960), "e");
        assert_eq!(subdivision_label(480, 960), "&");
        assert_eq!(subdivision_label(720, 960), "a");
        assert_eq!(subdivision_label(320, 960), "⅓");
        assert_eq!(subdivision_label(192, 960), "+1/5");
    }
}

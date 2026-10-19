use super::{measure::Measure, performance::Performance};

/// One end of a loop being picked out, on the performance timeline
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LoopPoint {
    /// A note, which the loop takes in whichever end of it it's at
    Note(u32),
    /// A place between notes, such as a bar line, where the loop can start
    /// or stop
    Edge(u32),
}

impl LoopPoint {
    pub fn timestamp(self) -> u32 {
        match self {
            LoopPoint::Note(timestamp) | LoopPoint::Edge(timestamp) => timestamp,
        }
    }
}

/// A stretch of the performance that plays over and over. Timestamps are on
/// the performance timeline, so a loop can be one particular pass through a
/// repeat
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LoopRange {
    /// Where the loop starts
    pub start: u32,
    /// Where playback goes back to `start`, which isn't played itself
    pub end: u32,
}

impl LoopRange {
    /// A loop between two points, in either order. Ending on a note takes in
    /// the whole of it, up to the next snap point of the performance
    pub fn between(
        a: LoopPoint,
        b: LoopPoint,
        performance: &Performance,
        measures: &[Measure],
    ) -> Self {
        // An edge at a note's time comes just before the note
        let order = |point: LoopPoint| (point.timestamp(), matches!(point, LoopPoint::Note(_)));
        let (first, second) = if order(a) <= order(b) { (a, b) } else { (b, a) };
        let snap_points = performance.snap_points(measures);
        let after = |timestamp: u32| {
            snap_points
                .iter()
                .copied()
                .find(|&snap| snap > timestamp)
                .unwrap_or_else(|| performance.end_timestamp())
                .max(timestamp + 1)
        };
        let start = first.timestamp();
        let end = match second {
            LoopPoint::Note(timestamp) => after(timestamp),
            LoopPoint::Edge(timestamp) if timestamp > start => timestamp,
            LoopPoint::Edge(_) => after(start),
        };
        Self { start, end }
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
        Some(Self::between(
            LoopPoint::Edge(first.timestamp),
            LoopPoint::Edge(last.timestamp + last.length),
            performance,
            measures,
        ))
    }

    pub fn contains(&self, timestamp: u32) -> bool {
        self.start <= timestamp && timestamp < self.end
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

impl LoopRange {
    /// The parts of the written score the loop plays, each once however many
    /// times it's played, sorted and without overlaps. A loop over several
    /// passes of a repeat plays the repeated measures more than once
    pub fn written_coverage(&self, performance: &Performance) -> Vec<(u32, u32)> {
        let mut spans = self.written_spans(performance);
        spans.sort_unstable();
        let mut coverage: Vec<(u32, u32)> = vec![];
        for (start, end) in spans {
            match coverage.last_mut() {
                Some((_, last_end)) if start <= *last_end => *last_end = (*last_end).max(end),
                _ => coverage.push((start, end)),
            }
        }
        coverage
    }
}

/// Where a performance timestamp falls, counted as a drummer would: the
/// played measure's number, the beat, and how far into the beat, e.g. "17:2"
/// for measure 17 beat 2, or "20:4&" for the and of beat 4. The very end of
/// the score is "End"
pub fn count_label(timestamp: u32, performance: &Performance, measures: &[Measure]) -> String {
    if timestamp >= performance.end_timestamp() {
        return "End".into();
    }
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
        note::{Duration, Note, NoteValue},
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
                            annotation: None,
                        })
                        .collect(),
                    time_signature: TimeSignature {
                        notes_per_measure: 4,
                        note_value: NoteValue::Quarter,
                    },
                    repeat_start,
                    section: None,
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

    use LoopPoint::{Edge, Note as N};

    #[test]
    fn note_loop_runs_to_the_note_after_the_last() {
        let measures = measures(&[(false, None); 4]);
        let performance = performance(&measures);
        // Dragged backwards, from beat 4& of measure 3 to beat 2 of measure 1
        let range = LoopRange::between(
            N(2 * MEASURE + 7 * EIGHTH),
            N(2 * EIGHTH),
            &performance,
            &measures,
        );
        assert_eq!(range.start, 2 * EIGHTH);
        assert_eq!(range.end, 3 * MEASURE);
        assert!(range.contains(range.start));
        assert!(!range.contains(range.end));
        assert_eq!(count_label(range.start, &performance, &measures), "1:2");
        assert_eq!(count_label(range.end, &performance, &measures), "4:1");
    }

    #[test]
    fn note_loop_mid_measure_ends_at_the_next_note() {
        let measures = measures(&[(false, None); 2]);
        let performance = performance(&measures);
        let range = LoopRange::between(N(0), N(3 * EIGHTH), &performance, &measures);
        assert_eq!(range.end, 4 * EIGHTH);
        assert_eq!(count_label(range.end, &performance, &measures), "1:3");
    }

    #[test]
    fn edge_at_a_bar_line_ends_the_loop_there() {
        let measures = measures(&[(false, None); 3]);
        let performance = performance(&measures);
        // From the first note of measure 2 to the bar line closing it, which
        // is where measure 3's first note is played: that note isn't looped
        let range = LoopRange::between(N(MEASURE), Edge(2 * MEASURE), &performance, &measures);
        assert_eq!(
            range,
            LoopRange {
                start: MEASURE,
                end: 2 * MEASURE
            }
        );
        // A note at the same time as an edge comes after it
        let range = LoopRange::between(Edge(2 * MEASURE), N(2 * MEASURE), &performance, &measures);
        assert_eq!(range.end, 2 * MEASURE + EIGHTH);
        // Edges in the same place still loop something
        let range = LoopRange::between(Edge(MEASURE), Edge(MEASURE), &performance, &measures);
        assert_eq!(range.end, MEASURE + EIGHTH);
    }

    #[test]
    fn loop_to_the_end_of_the_score() {
        let measures = measures(&[(false, None); 2]);
        let performance = performance(&measures);
        let last = 2 * MEASURE - EIGHTH;
        let range = LoopRange::between(N(last), N(last), &performance, &measures);
        assert_eq!(range.end, 2 * MEASURE);
        assert_eq!(count_label(range.end, &performance, &measures), "End");
    }

    #[test]
    fn measure_loop_covers_whole_measures() {
        let measures = measures(&[(false, None); 4]);
        let performance = performance(&measures);
        let range = LoopRange::between_measures(2, 1, &performance, &measures).unwrap();
        assert_eq!(
            range,
            LoopRange {
                start: MEASURE,
                end: 3 * MEASURE
            }
        );
        assert_eq!(count_label(range.start, &performance, &measures), "2:1");
        assert_eq!(count_label(range.end, &performance, &measures), "4:1");
        assert_eq!(
            LoopRange::between_measures(1, 9, &performance, &measures),
            None
        );
    }

    #[test]
    fn coverage_is_partial_where_the_loop_starts_and_ends_mid_measure() {
        let measures = measures(&[(false, None); 4]);
        let performance = performance(&measures);
        let range = LoopRange::between(
            N(2 * EIGHTH),
            N(2 * MEASURE + 3 * EIGHTH),
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
        let range = LoopRange::between(
            N(MEASURE + 4 * EIGHTH),
            N(2 * MEASURE + 3 * EIGHTH),
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
    fn coverage_tints_each_written_measure_once() {
        // Measure 0, then 1 played three times, then 2
        let measures = measures(&[(false, None), (true, Some(3)), (false, None)]);
        let performance = performance(&measures);
        // From halfway through measure 0, through every pass, into measure 2
        let range = LoopRange::between(
            N(4 * EIGHTH),
            N(4 * MEASURE + EIGHTH),
            &performance,
            &measures,
        );
        assert_eq!(range.written_spans(&performance).len(), 3);
        assert_eq!(
            range.written_coverage(&performance),
            vec![(4 * EIGHTH, 2 * MEASURE + 2 * EIGHTH)]
        );

        // The end of one pass and the start of the next leave a gap between
        let range = LoopRange::between(
            N(2 * MEASURE - EIGHTH),
            N(2 * MEASURE + EIGHTH),
            &performance,
            &measures,
        );
        assert_eq!(
            range.written_coverage(&performance),
            vec![
                (MEASURE, MEASURE + 2 * EIGHTH),
                (2 * MEASURE - EIGHTH, 2 * MEASURE)
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

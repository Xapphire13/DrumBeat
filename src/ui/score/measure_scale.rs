use crate::data::{
    drum_piece::DrumPiece,
    measure::Measure,
    note::{GraceNote, TICKS_PER_QUARTER},
    position::Position,
};

/// Space before the first note of a measure, in line heights
pub const LEFT_PADDING: f32 = 1.5;
/// Space after the last note's spacing in a measure, in line heights
pub const RIGHT_PADDING: f32 = 0.5;
/// Extra space at the end of a measure that closes a repeat, for its dots
/// and play count
const REPEAT_END_PADDING: f32 = 2.0;
/// Extra space at the start of a measure that opens a repeat, for its dots
const REPEAT_START_PADDING: f32 = 0.5;

/// Space before the first note and after the last note's spacing, in line
/// heights
fn padding(measure: &Measure) -> (f32, f32) {
    let left = LEFT_PADDING
        + if measure.repeat_start {
            REPEAT_START_PADDING
        } else {
            0.0
        };
    let right = RIGHT_PADDING
        + if measure.repeat_end.is_some() {
            REPEAT_END_PADDING
        } else {
            0.0
        };
    (left, right)
}

/// Space between grace notes, and from the last one to its main note, in
/// glyph sizes
pub const GRACE_NOTE_SPACING: f32 = 0.9;

/// Size of note heads and other glyphs, relative to the line height. Less
/// than one so heads half a line apart don't overlap
const GLYPH_SCALE: f32 = 0.72;

/// The size glyphs are drawn at, and the unit for horizontal spacing
pub fn glyph_size(line_height: f32) -> f32 {
    GLYPH_SCALE * line_height
}

#[derive(Clone, PartialEq)]
pub struct MeasureScale {
    pub line_height: f32,
    /// See `glyph_size`
    pub unit: f32,
    pub outer_x_bounds: (f32, f32),

    y_baseline: f32,
    /// `(timestamp, x)` for the measure start, every note onset and the
    /// measure end, sorted by timestamp. Positions in between are interpolated
    anchors: Vec<(u32, f32)>,
}

impl MeasureScale {
    pub fn new(
        outer_x_bounds: (f32, f32),
        y_baseline: f32,
        line_height: f32,
        measure: &Measure,
    ) -> MeasureScale {
        let (left_padding, right_padding) = padding(measure);
        let unit = glyph_size(line_height);
        let x_start = outer_x_bounds.0 + left_padding * unit;
        let x_end = outer_x_bounds.1 - right_padding * unit;
        let natural_offsets = natural_offsets(measure, unit);
        let natural_width = natural_offsets.last().map_or(0.0, |&(_, x)| x);
        let stretch = if natural_width > 0.0 {
            (x_end - x_start) / natural_width
        } else {
            0.0
        };

        let mut scale = MeasureScale {
            line_height,
            unit,
            outer_x_bounds,
            y_baseline,
            anchors: natural_offsets
                .into_iter()
                .map(|(timestamp, x)| (timestamp, x_start + x * stretch))
                .collect(),
        };
        scale.anchor_grace_notes(&measure.grace_notes);
        scale
    }

    /// Anchor each grace note's played time to where it is drawn, so the
    /// playhead crosses it as it plays. One played before the measure
    /// starts, or that would run backwards, is left out
    fn anchor_grace_notes(&mut self, grace_notes: &[GraceNote]) {
        let anchors: Vec<(u32, f32)> = grace_notes
            .iter()
            .map(|grace| (grace.timestamp, self.get_grace_x(grace)))
            .collect();
        for (timestamp, x) in anchors {
            let index = self.anchors.partition_point(|&(t, _)| t < timestamp);
            let fits = index > 0
                && self.anchors.get(index).is_some_and(|&(next_t, next_x)| {
                    let (prev_t, prev_x) = self.anchors[index - 1];
                    prev_t < timestamp && timestamp < next_t && prev_x < x && x < next_x
                });
            if fits {
                self.anchors.insert(index, (timestamp, x));
            }
        }
    }

    /// The same scale, with glyphs drawn `factor` times the size
    pub fn with_glyph_scale(&self, factor: f32) -> MeasureScale {
        MeasureScale {
            unit: self.unit * factor,
            ..self.clone()
        }
    }

    /// Where a grace note is drawn: a little ahead of its main note
    pub fn get_grace_x(&self, grace: &GraceNote) -> f32 {
        self.get_x(grace.main_timestamp)
            - f32::from(grace.steps_before_main) * GRACE_NOTE_SPACING * self.unit
    }

    pub fn get_y(&self, line_number: f32) -> f32 {
        // Note: we subtract from the baseline because SVG's draw from the top
        self.y_baseline - self.line_height * line_number
    }

    pub fn get_x(&self, timestamp: u32) -> f32 {
        let Some(&(first_timestamp, first_x)) = self.anchors.first() else {
            return self.outer_x_bounds.0;
        };
        if timestamp <= first_timestamp {
            return first_x;
        }

        self.anchors
            .windows(2)
            .find(|pair| timestamp < pair[1].0)
            .map_or_else(
                || self.anchors.last().map_or(first_x, |&(_, x)| x),
                |pair| {
                    let ((t0, x0), (t1, x1)) = (pair[0], pair[1]);
                    x0 + (x1 - x0) * (timestamp - t0) as f32 / (t1 - t0) as f32
                },
            )
    }

    /// Where the playhead is drawn at a timestamp. Like `get_x`, but the
    /// measure's end is drawn at `end_x`, so the playhead glides across the
    /// bar line to where the next measure starts instead of jumping there
    pub fn get_playhead_x(&self, timestamp: u32, end_x: f32) -> f32 {
        let (Some(&(end_timestamp, _)), Some(last_onset)) = (
            self.anchors.last(),
            self.anchors.len().checked_sub(2).map(|i| self.anchors[i]),
        ) else {
            return self.get_x(timestamp);
        };
        let (onset_timestamp, onset_x) = last_onset;
        if timestamp <= onset_timestamp {
            return self.get_x(timestamp);
        }

        let progress = (timestamp.min(end_timestamp) - onset_timestamp) as f32
            / (end_timestamp - onset_timestamp) as f32;
        onset_x + (end_x - onset_x) * progress
    }

    /// The inverse of `get_x`: the timestamp drawn at `x`
    pub fn get_timestamp(&self, x: f32) -> u32 {
        let Some(&(first_timestamp, first_x)) = self.anchors.first() else {
            return 0;
        };
        if x <= first_x {
            return first_timestamp;
        }

        self.anchors
            .windows(2)
            .find(|pair| x < pair[1].1)
            .map_or_else(
                || self.anchors.last().map_or(first_timestamp, |&(t, _)| t),
                |pair| {
                    let ((t0, x0), (t1, x1)) = (pair[0], pair[1]);
                    t0 + ((x - x0) / (x1 - x0) * (t1 - t0) as f32).round() as u32
                },
            )
    }

    pub fn get_position(&self, timestamp: u32, drum_piece: DrumPiece) -> Position {
        Position {
            x: self.get_x(timestamp),
            y: self.get_y(Self::get_line_number(drum_piece)),
        }
    }

    /// The staff line a drum piece's note head is drawn on
    pub fn get_line_number(drum_piece: DrumPiece) -> f32 {
        match drum_piece {
            DrumPiece::HihatPedal => -1.0,
            DrumPiece::Kick => 0.0,
            DrumPiece::FloorTom => 1.0,
            DrumPiece::Snare => 2.0,
            DrumPiece::Tom2 => 2.5,
            DrumPiece::Tom1 => 3.0,
            DrumPiece::Ride => 3.5,
            DrumPiece::Hihat => 4.0,
            DrumPiece::Crash => 4.5,
            DrumPiece::Splash => 5.0,
            DrumPiece::China => 5.5,
        }
    }
}

/// The width a measure needs to fit its notes comfortably, before it is
/// stretched to fill a staff
pub fn natural_width(measure: &Measure, line_height: f32) -> f32 {
    let unit = glyph_size(line_height);
    let notes_width = natural_offsets(measure, unit)
        .last()
        .map_or(0.0, |&(_, x)| x);

    let (left_padding, right_padding) = padding(measure);
    (left_padding + right_padding) * unit + notes_width
}

/// `(timestamp, x offset)` for the measure start, every note and rest onset
/// and the measure end, where each gap between onsets gets space based on its length
fn natural_offsets(measure: &Measure, unit: f32) -> Vec<(u32, f32)> {
    let end = measure.timestamp + measure.get_length();
    let mut timestamps: Vec<u32> = std::iter::once(measure.timestamp)
        .chain(
            measure
                .notes
                .iter()
                .map(|note| note.timestamp)
                .chain(measure.rests.iter().map(|rest| rest.timestamp))
                .filter(|&timestamp| timestamp > measure.timestamp && timestamp < end),
        )
        .chain(std::iter::once(end))
        .collect();
    timestamps.sort_unstable();
    timestamps.dedup();

    // Grace notes need room ahead of their main note
    let grace_room = |timestamp: u32| {
        measure
            .grace_notes
            .iter()
            .filter(|grace| grace.main_timestamp == timestamp)
            .map(|grace| f32::from(grace.steps_before_main) * GRACE_NOTE_SPACING * unit)
            .fold(0.0, f32::max)
    };
    let mut x = grace_room(measure.timestamp);
    let mut offsets = vec![(measure.timestamp, x)];
    for pair in timestamps.windows(2) {
        x += gap_width(pair[1] - pair[0], unit) + grace_room(pair[1]);
        offsets.push((pair[1], x));
    }
    offsets
}

/// Space for a gap between note onsets. Longer gaps get more space, but
/// grow logarithmically so long notes don't take over the staff
fn gap_width(gap: u32, unit: f32) -> f32 {
    let sixteenths = gap as f32 / (TICKS_PER_QUARTER / 4) as f32;
    let sixteenth_width = 1.6 * unit;
    let min_width = 1.2 * unit;

    (sixteenth_width * (1.0 + 0.6 * sixteenths.log2())).max(min_width)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::measure::Fill;
    use crate::data::{
        note::{Articulation, Duration, Note, NoteValue},
        time_signature::TimeSignature,
    };

    fn measure(timestamp: u32, onsets: &[u32]) -> Measure {
        Measure {
            timestamp,
            notes: onsets
                .iter()
                .map(|&onset| Note {
                    duration: Duration::new(NoteValue::Sixteenth),
                    midi_note: 38,
                    timestamp: timestamp + onset,
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
            repeat_start: false,
            rests: vec![],
            grace_notes: vec![],
            repeat_end: None,
            fill: Fill::Full,
        }
    }

    #[test]
    fn busier_measures_are_wider() {
        let quarters = measure(0, &[0, 960, 1920, 2880]);
        let sixteenths = measure(0, &(0..16).map(|i| i * 240).collect::<Vec<_>>());
        let empty = measure(0, &[]);

        assert!(natural_width(&sixteenths, 15.0) > natural_width(&quarters, 15.0));
        assert!(natural_width(&quarters, 15.0) > natural_width(&empty, 15.0));
    }

    #[test]
    fn notes_land_on_their_anchors() {
        let measure = measure(3840, &[0, 240, 480, 1920]);
        let scale = MeasureScale::new((100.0, 400.0), 0.0, 15.0, &measure);

        let first = scale.get_x(3840);
        let second = scale.get_x(3840 + 240);
        let third = scale.get_x(3840 + 480);
        let unit = glyph_size(15.0);
        assert!((first - (100.0 + LEFT_PADDING * unit)).abs() < 1e-3);
        assert!((scale.get_x(3840 + 3840) - (400.0 - RIGHT_PADDING * unit)).abs() < 1e-3);
        // Equal gaps get equal space, and positions round trip
        assert!(((second - first) - (third - second)).abs() < 1e-3);
        assert_eq!(scale.get_timestamp(third), 3840 + 480);
    }

    #[test]
    fn playhead_glides_to_the_next_measure() {
        let measure = measure(0, &[0, 1920]);
        let scale = MeasureScale::new((100.0, 400.0), 0.0, 15.0, &measure);
        let next_start = 420.0;

        // Up to the last onset it follows the notes
        assert!((scale.get_playhead_x(960, next_start) - scale.get_x(960)).abs() < 1e-3);
        assert!((scale.get_playhead_x(1920, next_start) - scale.get_x(1920)).abs() < 1e-3);
        // Then heads for the next measure's start, reaching it as the measure ends
        let halfway = scale.get_playhead_x(2880, next_start);
        assert!((halfway - (scale.get_x(1920) + next_start) / 2.0).abs() < 1e-3);
        assert!((scale.get_playhead_x(3840, next_start) - next_start).abs() < 1e-3);
    }

    #[test]
    fn grace_notes_get_room_and_are_crossed_as_they_play() {
        use crate::data::note::GraceNote;

        let plain = measure(0, &[0, 960, 1920, 2880]);
        let mut flams = plain.clone();
        flams.grace_notes = [0_u32, 960]
            .into_iter()
            .map(|main_timestamp| GraceNote {
                midi_note: 38,
                timestamp: main_timestamp.saturating_sub(60),
                main_timestamp,
                steps_before_main: 1,
                voice: 0,
                articulation: Articulation::Normal,
            })
            .collect();
        assert!(natural_width(&flams, 15.0) > natural_width(&plain, 15.0));

        let width = natural_width(&flams, 15.0);
        let scale = MeasureScale::new((0.0, width), 0.0, 15.0, &flams);
        let unit = glyph_size(15.0);
        // The first flam clears the bar line by the usual padding
        let first = &flams.grace_notes[0];
        assert!((scale.get_grace_x(first) - LEFT_PADDING * unit).abs() < 1e-3);
        // The second is where the playhead is when it plays
        let second = &flams.grace_notes[1];
        assert!((scale.get_x(second.timestamp) - scale.get_grace_x(second)).abs() < 1e-3);
        assert!(scale.get_grace_x(second) < scale.get_x(960));
    }
}

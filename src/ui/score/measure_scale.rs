use crate::data::{
    drum_piece::DrumPiece,
    measure::Measure,
    note::{NoteValue, TICKS_PER_QUARTER},
    position::Position,
};

/// Space before the first note of a measure, in line heights
pub const LEFT_PADDING: f32 = 1.5;
/// Space after the last note's spacing in a measure, in line heights
pub const RIGHT_PADDING: f32 = 0.5;

#[derive(Clone, PartialEq)]
pub struct MeasureScale {
    pub line_height: f32,
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
        let x_start = outer_x_bounds.0 + LEFT_PADDING * line_height;
        let x_end = outer_x_bounds.1 - RIGHT_PADDING * line_height;
        let natural_offsets = natural_offsets(measure, line_height);
        let natural_width = natural_offsets.last().map_or(0.0, |&(_, x)| x);
        let stretch = if natural_width > 0.0 {
            (x_end - x_start) / natural_width
        } else {
            0.0
        };

        MeasureScale {
            line_height,
            outer_x_bounds,
            y_baseline,
            anchors: natural_offsets
                .into_iter()
                .map(|(timestamp, x)| (timestamp, x_start + x * stretch))
                .collect(),
        }
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

    pub fn get_beam_line_number(&self, note_value: NoteValue) -> f32 {
        match note_value {
            NoteValue::Whole | NoteValue::Half | NoteValue::Quarter | NoteValue::Eighth => 6.,
            NoteValue::Sixteenth => 6.5,
            NoteValue::ThirtySecond => 7.,
            NoteValue::SixtyFourth => 7.5,
        }
    }

    fn get_line_number(drum_piece: DrumPiece) -> f32 {
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
    let notes_width = natural_offsets(measure, line_height)
        .last()
        .map_or(0.0, |&(_, x)| x);

    (LEFT_PADDING + RIGHT_PADDING) * line_height + notes_width
}

/// `(timestamp, x offset)` for the measure start, every note onset and the
/// measure end, where each gap between onsets gets space based on its length
fn natural_offsets(measure: &Measure, line_height: f32) -> Vec<(u32, f32)> {
    let end = measure.timestamp + measure.get_length();
    let mut timestamps: Vec<u32> = std::iter::once(measure.timestamp)
        .chain(
            measure
                .notes
                .iter()
                .map(|note| note.timestamp)
                .filter(|&timestamp| timestamp > measure.timestamp && timestamp < end),
        )
        .chain(std::iter::once(end))
        .collect();
    timestamps.sort_unstable();
    timestamps.dedup();

    let mut x = 0.0;
    let mut offsets = vec![(measure.timestamp, 0.0)];
    for pair in timestamps.windows(2) {
        x += gap_width(pair[1] - pair[0], line_height);
        offsets.push((pair[1], x));
    }
    offsets
}

/// Space for a gap between note onsets. Longer gaps get more space, but
/// grow logarithmically so long notes don't take over the staff
fn gap_width(gap: u32, line_height: f32) -> f32 {
    let sixteenths = gap as f32 / (TICKS_PER_QUARTER / 4) as f32;
    let sixteenth_width = 1.6 * line_height;
    let min_width = 1.2 * line_height;

    (sixteenth_width * (1.0 + 0.6 * sixteenths.log2())).max(min_width)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{
        note::{Duration, Note},
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
                })
                .collect(),
            time_signature: TimeSignature {
                notes_per_measure: 4,
                note_value: NoteValue::Quarter,
            },
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
        assert!((first - (100.0 + LEFT_PADDING * 15.0)).abs() < 1e-3);
        assert!((scale.get_x(3840 + 3840) - (400.0 - RIGHT_PADDING * 15.0)).abs() < 1e-3);
        // Equal gaps get equal space, and positions round trip
        assert!(((second - first) - (third - second)).abs() < 1e-3);
        assert_eq!(scale.get_timestamp(third), 3840 + 480);
    }
}

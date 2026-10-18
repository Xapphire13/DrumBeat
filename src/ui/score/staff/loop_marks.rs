use dioxus::prelude::*;

use crate::{
    data::{
        loop_range::{LoopPoint, LoopRange},
        measure::Measure,
        performance::Performance,
    },
    ui::score::{measure_scale::MeasureScale, note::head_half_width},
};

/// Where the loop is drawn on one staff, in the staff's own units
#[derive(Clone, PartialEq, Debug, Default)]
pub struct LoopGeometry {
    /// The stretches of the staff the loop covers, from x to x
    pub pieces: Vec<(f32, f32)>,
    /// Where the loop starts, if it starts on this staff
    pub start_x: Option<f32>,
    /// Where the loop ends, if it ends on this staff
    pub end_x: Option<f32>,
}

impl LoopGeometry {
    pub fn new(
        range: LoopRange,
        performance: &Performance,
        measures: &[Measure],
        scales: &[MeasureScale],
    ) -> Self {
        let spans = range.written_spans(performance);
        let coverage = range.written_coverage(performance);
        let mut geometry = LoopGeometry::default();
        let (Some(&(loop_start, _)), Some(&(_, loop_end))) = (spans.first(), spans.last()) else {
            return geometry;
        };

        for (measure, scale) in measures.iter().zip(scales) {
            let (measure_start, measure_end) = (measure.timestamp, measure.end_timestamp());
            // Tinted once, however many passes of a repeat the loop plays
            for &(start, end) in &coverage {
                let (from, to) = (start.max(measure_start), end.min(measure_end));
                if from < to {
                    geometry
                        .pieces
                        .push((start_x(from, measure, scale), end_x(to, measure, scale)));
                }
            }
            if measure_start <= loop_start && loop_start < measure_end {
                geometry.start_x = Some(start_x(loop_start, measure, scale));
            }
            if measure_start < loop_end && loop_end <= measure_end {
                geometry.end_x = Some(end_x(loop_end, measure, scale));
            }
        }
        geometry
    }
}

/// A place on a staff that a loop's end can be dragged to
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct StaffPoint {
    /// Index of the measure on the staff
    pub measure: usize,
    /// Ticks into the measure, up to its length for its closing bar line
    pub offset: u32,
    /// Whether it's a note, rather than an edge between notes
    pub note: bool,
}

impl StaffPoint {
    /// Where the point is played, as the other end of a loop from `anchor`,
    /// which is written at `anchor_written` (see `Performance::locate`).
    /// `first_measure` is the index in the score of the staff's first measure
    pub fn locate(
        self,
        first_measure: usize,
        performance: &Performance,
        anchor: u32,
        anchor_written: u32,
    ) -> Option<LoopPoint> {
        let timestamp = performance.locate(
            first_measure + self.measure,
            self.offset,
            anchor,
            anchor_written,
        )?;
        Some(if self.note {
            LoopPoint::Note(timestamp)
        } else {
            LoopPoint::Edge(timestamp)
        })
    }
}

/// The place on the staff nearest `x` that a loop's end snaps to.
///
/// Dragging across the score snaps to notes, the start of each measure, and
/// the bar line closing each measure, so a loop can end with the measure
/// rather than taking in the next one's first note. Dragging a loop's tab
/// (`edges`) snaps to where the loop's edge would be drawn: just ahead of each
/// note, and the bar lines
pub fn closest_loop_point(
    x: f32,
    scales: &[MeasureScale],
    measures: &[Measure],
    edges: bool,
) -> Option<StaffPoint> {
    let mut candidates: Vec<(StaffPoint, f32)> = vec![];
    for (i, (measure, scale)) in measures.iter().zip(scales).enumerate() {
        let mut onsets: Vec<u32> = std::iter::once(measure.timestamp)
            .chain(measure.notes.iter().map(|note| note.timestamp))
            .collect();
        onsets.sort_unstable();
        onsets.dedup();
        for timestamp in onsets {
            let point = StaffPoint {
                measure: i,
                offset: timestamp - measure.timestamp,
                note: !edges,
            };
            let x = if edges {
                start_x(timestamp, measure, scale)
            } else {
                scale.get_x(timestamp)
            };
            candidates.push((point, x));
        }
        let closing = StaffPoint {
            measure: i,
            offset: measure.get_length(),
            note: false,
        };
        candidates.push((closing, scale.outer_x_bounds.1));
    }

    candidates
        .into_iter()
        .min_by(|(_, a), (_, b)| (a - x).abs().total_cmp(&(b - x).abs()))
        .map(|(point, _)| point)
}

/// A little ahead of a note, so the loop takes in its head
fn lead(scale: &MeasureScale) -> f32 {
    head_half_width(scale.unit) + 0.3 * scale.unit
}

/// Where a loop starting at `timestamp` is drawn from: the bar line at the
/// start of a measure, or just ahead of a note
fn start_x(timestamp: u32, measure: &Measure, scale: &MeasureScale) -> f32 {
    if timestamp <= measure.timestamp {
        scale.outer_x_bounds.0
    } else {
        scale.get_x(timestamp) - lead(scale)
    }
}

/// Where a loop ending at `timestamp` is drawn to: the bar line at the end of
/// a measure, or just ahead of the note that follows the loop
fn end_x(timestamp: u32, measure: &Measure, scale: &MeasureScale) -> f32 {
    if timestamp >= measure.end_timestamp() {
        scale.outer_x_bounds.1
    } else {
        scale.get_x(timestamp) - lead(scale)
    }
}

/// The loop's tint behind the staff, with a solid edge where it starts and
/// ends
#[component]
pub fn LoopMarks(geometry: LoopGeometry, height: f32, zoom: f32) -> Element {
    // Edges are the same width at every zoom
    let edge_width = 2.0 / zoom;

    rsx! {
        for (x1 , x2) in geometry.pieces {
            rect {
                class: "loop-tint",
                x: x1,
                y: 0,
                width: (x2 - x1).max(0.0),
                height,
            }
        }
        for x in geometry.start_x.into_iter().chain(geometry.end_x) {
            line {
                class: "loop-edge",
                stroke_width: edge_width,
                x1: x,
                y1: 0,
                x2: x,
                y2: height,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{
        measure::Fill,
        note::{Articulation, Duration, Note, NoteValue},
        time_signature::TimeSignature,
    };

    /// A 4/4 measure of eighth notes, drawn from x 0 to 400
    fn measure() -> (Measure, MeasureScale) {
        let measure = Measure {
            timestamp: 0,
            notes: (0..8)
                .map(|n| Note {
                    duration: Duration::new(NoteValue::Eighth),
                    midi_note: 38,
                    timestamp: n * 480,
                    tied: false,
                    voice: 0,
                    articulation: Articulation::Normal,
                    annotation: None,
                })
                .collect(),
            rests: vec![],
            grace_notes: vec![],
            time_signature: TimeSignature {
                notes_per_measure: 4,
                note_value: NoteValue::Quarter,
            },
            repeat_start: false,
            repeat_end: None,
            fill: Fill::Full,
        };
        let scale = MeasureScale::new((0.0, 400.0), 100.0, 20.0, &measure);
        (measure, scale)
    }

    #[test]
    fn dragging_onto_the_bar_line_ends_the_loop_with_the_measure() {
        let (measure, scale) = measure();
        let (measures, scales) = (vec![measure], vec![scale.clone()]);
        let last_note = scale.get_x(7 * 480);

        let point = closest_loop_point(last_note, &scales, &measures, false).unwrap();
        assert_eq!((point.offset, point.note), (7 * 480, true));
        let point = closest_loop_point(399.0, &scales, &measures, false).unwrap();
        assert_eq!((point.offset, point.note), (3840, false));
    }

    #[test]
    fn tabs_snap_to_the_edges_the_loop_is_drawn_at() {
        let (measure, scale) = measure();
        let (measures, scales) = (vec![measure.clone()], vec![scale.clone()]);
        // Just ahead of the last note, where a loop ending there is drawn
        let edge = start_x(7 * 480, &measure, &scale);
        let point = closest_loop_point(edge + 1.0, &scales, &measures, true).unwrap();
        assert_eq!((point.offset, point.note), (7 * 480, false));
        let point = closest_loop_point(1.0, &scales, &measures, true).unwrap();
        assert_eq!((point.offset, point.note), (0, false));
    }
}

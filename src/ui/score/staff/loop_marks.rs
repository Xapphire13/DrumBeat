use dioxus::prelude::*;

use crate::{
    data::{loop_range::LoopRange, measure::Measure, performance::Performance},
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
        let mut geometry = LoopGeometry::default();
        let (Some(&(loop_start, _)), Some(&(_, loop_end))) = (spans.first(), spans.last()) else {
            return geometry;
        };

        for (measure, scale) in measures.iter().zip(scales) {
            let (measure_start, measure_end) = (measure.timestamp, measure.end_timestamp());
            for &(start, end) in &spans {
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

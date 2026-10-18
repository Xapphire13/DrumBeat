use dioxus::prelude::*;

use crate::data::{
    drum_piece::DrumPiece,
    note::{Articulation, NoteValue},
    position::Position,
};

use super::measure_scale::MeasureScale;

/// Half the width of a note head
pub fn head_half_width(unit: f32) -> f32 {
    (unit - 2.) / 2.
}

/// The top and bottom staff lines, as line numbers
const TOP_STAFF_LINE: f32 = 3.5;
const BOTTOM_STAFF_LINE: f32 = -0.5;

/// The short extra staff lines a note above or below the staff needs, out to
/// the note's own line or the space it sits in
fn ledger_lines(line_number: f32) -> Vec<f32> {
    let mut lines = vec![];
    let mut line = TOP_STAFF_LINE + 1.0;
    while line <= line_number + 1e-3 {
        lines.push(line);
        line += 1.0;
    }
    let mut line = BOTTOM_STAFF_LINE - 1.0;
    while line >= line_number - 1e-3 {
        lines.push(line);
        line -= 1.0;
    }
    lines
}

/// A note head, with any ledger lines it needs. Stems and beams are drawn
/// separately, per chord
#[component]
pub fn Note(
    value: NoteValue,
    drum_piece: DrumPiece,
    articulation: Articulation,
    position: Position,
    scale: MeasureScale,
) -> Element {
    let Position { x, y } = position;
    let size = head_half_width(scale.unit);
    let fill_color = "black";
    let is_hollow = matches!(value, NoteValue::Whole | NoteValue::Half);
    let ledger_half_width = 0.85 * scale.unit;
    let ledger_ys: Vec<f32> = ledger_lines(MeasureScale::get_line_number(drum_piece))
        .into_iter()
        .map(|line| scale.get_y(line))
        .collect();

    rsx! {
        g {
            for ledger_y in ledger_ys {
                line {
                    x1: x - ledger_half_width,
                    y1: ledger_y,
                    x2: x + ledger_half_width,
                    y2: ledger_y,
                    stroke: fill_color,
                    stroke_width: 1,
                }
            }
            if articulation == Articulation::RideBell {
                // Hollow diamond head for the ride's bell. Its stem starts
                // from the right (or, stem down, left) vertex
                polygon {
                    points: "{x - size},{y} {x},{y - size} {x + size},{y} {x},{y + size}",
                    fill: "white",
                    stroke: fill_color,
                    stroke_width: 1.5,
                }
            } else if drum_piece.is_cymbal() {
                // Note body - X shape
                line {
                    x1: x - size,
                    y1: y - size,
                    x2: x + size,
                    y2: y + size,
                    stroke: fill_color,
                    stroke_width: 1.5,
                }
                line {
                    x1: x - size,
                    y1: y + size,
                    x2: x + size,
                    y2: y - size,
                    stroke: fill_color,
                    stroke_width: 1.5,
                }
            } else {
                // Note body - ellipse (tilted music note)
                ellipse {
                    rx: scale.unit / 2.0,
                    ry: scale.unit * 0.3,
                    cx: x,
                    cy: y,
                    transform: format!("rotate(-30 {x} {y})"),
                    fill: if is_hollow { "white" } else { fill_color },
                    stroke: fill_color,
                    stroke_width: 1.5,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ledger_lines_reach_the_note() {
        // In the staff, or the space just outside it: none
        assert!(ledger_lines(2.0).is_empty());
        assert!(ledger_lines(4.0).is_empty());
        assert!(ledger_lines(-1.0).is_empty());
        // On the first ledger line, or the space above it
        assert_eq!(ledger_lines(4.5), vec![4.5]);
        assert_eq!(ledger_lines(5.0), vec![4.5]);
        assert_eq!(ledger_lines(5.5), vec![4.5, 5.5]);
        assert_eq!(ledger_lines(-1.5), vec![-1.5]);
    }
}

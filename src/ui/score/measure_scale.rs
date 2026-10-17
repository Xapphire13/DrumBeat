use crate::data::{
    drum_piece::DrumPiece, note::NoteValue, position::Position, time_signature::TimeSignature,
};

#[derive(Clone, PartialEq)]
pub struct MeasureScale {
    pub line_height: f32,
    pub outer_x_bounds: (f32, f32),

    x_range: (f32, f32),
    y_baseline: f32,
    start_timestamp: u32,
    measure_length: u32,
}

impl MeasureScale {
    pub fn new(
        outer_x_bounds: (f32, f32),
        x_range: (f32, f32),
        y_baseline: f32,
        line_height: f32,
        start_timestamp: u32,
        time_signature: TimeSignature,
    ) -> MeasureScale {
        let ticks_per_note = time_signature.note_value.get_ticks();
        let measure_length = ticks_per_note * time_signature.notes_per_measure;

        MeasureScale {
            line_height,
            outer_x_bounds,
            x_range,
            y_baseline,
            start_timestamp,
            measure_length,
        }
    }

    pub fn get_y(&self, line_number: f32) -> f32 {
        // Note: we subtract from `self.height` because SVG's draw from the top
        self.y_baseline - self.line_height * line_number
    }

    pub fn get_x(&self, timestamp: u32) -> f32 {
        let step_size = (self.x_range.1 - self.x_range.0) / self.measure_length as f32;

        self.x_range.0 + (timestamp - self.start_timestamp) as f32 * step_size
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

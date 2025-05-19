use crate::data::{
    note::{Note, NoteValue, Surface},
    position::Position,
};

#[derive(Clone, PartialEq)]
pub struct MeasureScale {
    pub line_height: f32,

    x_range: (f32, f32),
    y_baseline: f32,
    start_timestamp: u32,
}

impl MeasureScale {
    pub fn new(
        x_range: (f32, f32),
        y_baseline: f32,
        line_height: f32,
        start_timestamp: u32,
    ) -> MeasureScale {
        MeasureScale {
            x_range,
            y_baseline,
            line_height,
            start_timestamp,
        }
    }

    pub fn get_y(&self, line_number: f32) -> f32 {
        // Note: we subtract from `self.height` because SVG's draw from the top
        self.y_baseline - self.line_height * line_number
    }

    pub fn get_position(&self, note: &Note) -> Position {
        let (x_min, x_max) = self.x_range;
        let width = x_max - x_min;

        let x_pos = x_min + (note.timestamp - self.start_timestamp) as f32 * width / 256.0;

        let y_pos = self.get_y(self.get_surface_line_number(&note.surface));

        Position { x: x_pos, y: y_pos }
    }

    pub fn get_beam_line_number(&self, note_value: NoteValue) -> f32 {
        match note_value {
            NoteValue::Quarter | NoteValue::Eighth => 6.,
            NoteValue::Sixteenth => 6.5,
            NoteValue::Thirtysecondth => 7.,
        }
    }

    fn get_surface_line_number(&self, surface: &Surface) -> f32 {
        match surface {
            Surface::Snare => 2.0,
            Surface::Hihat => 4.0,
            Surface::Tom1 => 3.0,
            Surface::Tom2 => 2.5,
            Surface::FloorTom => 1.0,
            Surface::Crash => 4.5,
            Surface::Ride => 3.5,
            Surface::Kick => 0.0,
        }
    }
}

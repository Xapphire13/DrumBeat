use crate::data::{
    note::{Note, NoteValue, Surface},
    position::Position,
};

#[derive(Clone, PartialEq)]
pub struct MeasureScale {
    x_range: (f32, f32),
    y_baseline: f32,
    pub line_height: f32,
}

impl MeasureScale {
    pub fn new(x_range: (f32, f32), y_baseline: f32, line_height: f32) -> MeasureScale {
        MeasureScale {
            x_range,
            y_baseline,
            line_height,
        }
    }

    pub fn get_y(&self, line_number: f32) -> f32 {
        self.y_baseline - self.line_height * line_number
    }

    pub fn get_position(&self, note: &Note) -> Position {
        let (x_min, x_max) = self.x_range;
        let width = x_max - x_min;
        let mut x_pos = x_min;
        let y_pos;

        match note.value {
            NoteValue::Quarter => x_pos += (note.offset as f32) * width / 4.,
            NoteValue::Eighth => x_pos += (note.offset as f32) * width / 8.,
            NoteValue::Sixteenth => x_pos += (note.offset as f32) * width / 16.,
            NoteValue::Thirtysecondth => x_pos += (note.offset as f32) * width / 32.,
        }

        // Note: we subtract from `self.height` because SVG's draw from the top
        match note.surface {
            Surface::Snare => y_pos = self.get_y(2.),
            Surface::Hihat => y_pos = self.get_y(4.),
            Surface::Tom1 => y_pos = self.get_y(3.),
            Surface::Tom2 => y_pos = self.get_y(2.5),
            Surface::FloorTom => y_pos = self.get_y(1.),
            Surface::Crash => y_pos = self.get_y(4.5),
            Surface::Ride => y_pos = self.get_y(3.5),
            Surface::Kick => y_pos = self.get_y(0.),
        }

        Position { x: x_pos, y: y_pos }
    }
}

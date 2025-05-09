use derive_builder::Builder;

use super::note::{Note, NoteValue};

#[derive(Clone, PartialEq, Builder)]
pub struct MeasureScale {
    x_range: (i32, i32),
    height: i32,
}

impl MeasureScale {
    pub fn get_position(&self, note: &Note) -> (i32, i32) {
        let (x_min, x_max) = self.x_range;
        let width = x_max - x_min;
        let mut x_pos = x_min;
        let mut y_pos = 0;

        match note.value {
            NoteValue::Quarter => x_pos += (note.offset as i32) * width / 4,
            NoteValue::Eighth => x_pos += (note.offset as i32) * width / 8,
            NoteValue::Sixteenth => x_pos += (note.offset as i32) * width / 16,
            NoteValue::Thirtysecondth => x_pos += (note.offset as i32) * width / 32,
        }

        // TODO, calculate actual position
        // Note: we subtract from `self.height` because SVG's draw from the top
        match note.surface {
            super::note::Surface::Snare => {
                y_pos = self.height - ((self.height / 6) as f32 * 3.5) as i32
            }
            super::note::Surface::Highhat => y_pos = self.height - (self.height / 6) * 5,
            super::note::Surface::Tom1 => y_pos = self.height - (self.height / 6) * 3,
            super::note::Surface::Tom2 => y_pos = self.height - (self.height / 6) * 2,
            super::note::Surface::FloorTom => y_pos = self.height - (self.height / 6) * 1,
            super::note::Surface::Crash => {
                y_pos = self.height - ((self.height / 6) as f32 * 0.5) as i32
            }
            super::note::Surface::Ride => y_pos = self.height - (self.height / 6) * 5,
        }

        (x_pos, y_pos)
    }
}

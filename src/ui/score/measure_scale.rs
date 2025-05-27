use crate::data::{
    note::{Note, NoteValue, Surface},
    position::Position,
};

#[derive(Clone, PartialEq)]
pub struct MeasureScale {
    pub line_height: f32,
    pub outer_x_bounds: (f32, f32),

    x_range: (f32, f32),
    y_baseline: f32,
    start_timestamp: u32,
}

impl MeasureScale {
    pub fn new(
        outer_x_bounds: (f32, f32),
        x_range: (f32, f32),
        y_baseline: f32,
        line_height: f32,
        start_timestamp: u32,
    ) -> MeasureScale {
        MeasureScale {
            outer_x_bounds,
            x_range,
            y_baseline,
            line_height,
            start_timestamp,
        }
    }

    pub fn is_within_bounds(&self, Position { x, .. }: &Position) -> bool {
        *x >= self.outer_x_bounds.0 && *x <= self.outer_x_bounds.1
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
            NoteValue::ThirtySecond => 7.,
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

    fn get_surface_from_line_number(&self, line_number: f32) -> Option<Surface> {
        match line_number {
            2.0 => Some(Surface::Snare),
            4.0 => Some(Surface::Hihat),
            3.0 => Some(Surface::Tom1),
            2.5 => Some(Surface::Tom2),
            1.0 => Some(Surface::FloorTom),
            4.5 => Some(Surface::Crash),
            3.5 => Some(Surface::Ride),
            0.0 => Some(Surface::Kick),
            _ => None,
        }
    }

    pub fn get_nearest_note(&self, position: Position, note_value: NoteValue) -> Option<Note> {
        let Position { x, y } = position;
        let x = x - self.x_range.0; // We want the x-position to be in terms of offset from the first note position

        // Convert y-position to line number:
        // 1. Calculate distance from baseline (self.y_baseline - y)
        // 2. Divide by half line height to get double-precision line numbers
        // 3. Round to nearest half-line
        // 4. Divide by 2 to get our standard line numbering system
        let line_number = ((self.y_baseline - y) / (self.line_height / 2.0)).round() / 2.0;

        if let Some(surface) = self.get_surface_from_line_number(line_number) {
            let width = self.x_range.1 - self.x_range.0;
            let timestamp = self.start_timestamp
                + match note_value {
                    NoteValue::Quarter => {
                        // Divide measure into 4 quarters (each worth 64 256th notes)
                        let quarter_width = width / 4.0;
                        // Determine which quarter we're in:
                        // 1. Divide by quarter width to get quarter note index (0-3)
                        // 2. Round to nearest whole quarter note
                        // 3. Multiply by 64 to convert to 256th notes
                        (x / quarter_width).round() as u32 * 64
                    }
                    NoteValue::Eighth => {
                        // Divide measure into 8 eighths (each worth 32 256th notes)
                        let eighth_width = width / 8.0;
                        (x / eighth_width).round() as u32 * 32
                    }
                    NoteValue::Sixteenth => {
                        // Divide measure into 16 sixteenths (each worth 16 256th notes)
                        let sixteenth_width = width / 16.0;
                        (x / sixteenth_width).round() as u32 * 16
                    }
                    NoteValue::ThirtySecond => {
                        // Divide measure into 32 thirty-seconds (each worth 8 256th notes)
                        let thirtysecondth_width = width / 32.0;
                        (x / thirtysecondth_width).round() as u32 * 8
                    }
                };

            // Only return notes within the measure
            // TODO, update this when we support time-signatures other than 4/4
            if timestamp - self.start_timestamp < 256 {
                return Some(Note {
                    value: note_value,
                    surface,
                    timestamp,
                });
            }
        }

        None
    }
}

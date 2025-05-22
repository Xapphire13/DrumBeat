use std::fmt::Display;

use crate::data::note::NoteValue;

#[derive(Clone)]
pub enum AppMode {
    View,
    Edit { note_value: NoteValue },
}

impl Display for AppMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppMode::View => write!(f, "View"),
            AppMode::Edit { .. } => write!(f, "Edit"),
        }
    }
}

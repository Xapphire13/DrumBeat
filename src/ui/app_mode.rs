use std::fmt::Display;

#[derive(Clone)]
pub enum AppMode {
    View,
    Edit,
}

impl Display for AppMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppMode::View => write!(f, "View"),
            AppMode::Edit => write!(f, "Edit"),
        }
    }
}

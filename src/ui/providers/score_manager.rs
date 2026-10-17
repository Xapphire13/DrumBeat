use std::path::Path;

use dioxus::prelude::*;

use crate::{data::score::Score, gp};

/// Load a Guitar Pro 8 file into the app's score, falling back to the file
/// name when the score has no title
pub fn load_score(path: &Path) -> Result<Score, gp::GpError> {
    let mut score = gp::load(path)?.to_score()?;
    if score.title.is_empty() {
        score.title = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default();
    }
    Ok(score)
}

/// Provides the open score, starting with the file passed on the command
/// line, if any
pub fn use_score_manager_provider() -> Signal<Score> {
    use_context_provider(|| {
        let score = std::env::args_os()
            .nth(1)
            .and_then(|path| match load_score(Path::new(&path)) {
                Ok(score) => Some(score),
                Err(e) => {
                    eprintln!("Couldn't open {}: {e}", Path::new(&path).display());
                    None
                }
            })
            .unwrap_or_else(Score::empty);

        Signal::new(score)
    })
}

pub fn use_score_manager() -> Signal<Score> {
    use_context::<Signal<Score>>()
}

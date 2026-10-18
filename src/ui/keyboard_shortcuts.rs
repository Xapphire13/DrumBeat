use dioxus::prelude::*;
use serde::Deserialize;

use crate::ui::providers::{
    playback_manager::use_playback_manager, score_manager::use_score_manager,
};

/// Held with Left/Right to move by measure instead of by note. Option matches
/// how macOS moves by word in text, while Cmd+Left/Right means line start/end
const MEASURE_MODIFIER: Modifier = Modifier::Option;

#[allow(dead_code)]
enum Modifier {
    Option,
    Cmd,
}

/// Listens on the whole document, so the shortcuts work wherever focus is,
/// except while typing in a text or number field such as the BPM box. The
/// keys' default actions, like scrolling the score or nudging the tempo
/// slider, are suppressed
const LISTENER: &str = r#"
const keys = new Set([" ", "ArrowLeft", "ArrowRight", "Home", "End"]);
document.addEventListener("keydown", (event) => {
    if (!keys.has(event.key)) return;
    const target = event.target;
    if (target instanceof HTMLTextAreaElement) return;
    if (target instanceof HTMLInputElement && target.type !== "range") return;
    event.preventDefault();
    dioxus.send({
        key: event.key,
        alt: event.altKey,
        meta: event.metaKey,
        ctrl: event.ctrlKey,
        shift: event.shiftKey,
        repeat: event.repeat,
    });
});
"#;

#[derive(Deserialize)]
struct KeyPress {
    key: String,
    alt: bool,
    meta: bool,
    ctrl: bool,
    shift: bool,
    repeat: bool,
}

impl KeyPress {
    /// Whether exactly the measure modifier is held
    fn measure_modifier(&self) -> bool {
        let (alt, meta) = match MEASURE_MODIFIER {
            Modifier::Option => (true, false),
            Modifier::Cmd => (false, true),
        };
        !self.ctrl && !self.shift && self.alt == alt && self.meta == meta
    }

    fn no_modifiers(&self) -> bool {
        !(self.alt || self.meta || self.ctrl || self.shift)
    }
}

/// Space plays and pauses, Left/Right step between notes, the measure
/// modifier with Left/Right steps between measures, and Home/End jump to the
/// start and end of the score
pub fn use_keyboard_shortcuts() {
    let mut playback_manager = use_playback_manager();
    let score = use_score_manager();

    use_future(move || async move {
        let mut listener = document::eval(LISTENER);
        while let Ok(press) = listener.recv::<KeyPress>().await {
            let score = score.read();
            match press.key.as_str() {
                // Holding space shouldn't keep toggling
                " " if press.no_modifiers() && !press.repeat => {
                    if playback_manager.is_playing() {
                        playback_manager.pause();
                    } else {
                        playback_manager.play(&score);
                    }
                }
                "ArrowLeft" if press.measure_modifier() => {
                    playback_manager.skip_measures(-1, &score);
                }
                "ArrowRight" if press.measure_modifier() => {
                    playback_manager.skip_measures(1, &score);
                }
                "ArrowLeft" if press.no_modifiers() => {
                    playback_manager.skip_to_previous_note(&score)
                }
                "ArrowRight" if press.no_modifiers() => playback_manager.skip_to_next_note(&score),
                "Home" if press.no_modifiers() => playback_manager.skip_to_start(&score),
                "End" if press.no_modifiers() => playback_manager.skip_to_end(&score),
                _ => {}
            }
        }
    });
}

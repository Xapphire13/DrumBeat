#![warn(clippy::pedantic)]

use dioxus::prelude::*;
use lucide_dioxus::{FolderOpen, ZoomIn, ZoomOut};

use crate::ui::{
    keyboard_shortcuts::use_keyboard_shortcuts,
    providers::{
        loop_editor::use_loop_editor_provider,
        playback_manager::{use_playback_manager, use_playback_manager_provider},
        score_manager::{load_score, use_score_manager, use_score_manager_provider},
        zoom::{use_zoom, use_zoom_provider},
    },
    score::Score,
};

const MAIN_CSS: Asset = asset!("/assets/main.css");

#[component]
pub fn App() -> Element {
    use_playback_manager_provider();
    use_loop_editor_provider();
    use_score_manager_provider();
    use_zoom_provider();
    use_keyboard_shortcuts();

    rsx! {
        document::Link { rel: "stylesheet", href: MAIN_CSS }

        Toolbar {}

        Score {}
    }
}

#[component]
fn Toolbar() -> Element {
    let mut score = use_score_manager();
    let mut playback_manager = use_playback_manager();
    let mut error = use_signal(|| None::<String>);

    let handle_open = move |_: MouseEvent| {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Guitar Pro 8", &["gp"])
            .pick_file()
        else {
            return;
        };

        match load_score(&path) {
            Ok(new_score) => {
                playback_manager.reset();
                score.set(new_score);
                error.set(None);
            }
            Err(e) => error.set(Some(format!("Couldn't open {}: {e}", path.display()))),
        }
    };

    rsx! {
        div { class: "toolbar",
            button {
                class: "open-button",
                title: "Open a Guitar Pro 8 file",
                onclick: handle_open,
                FolderOpen { size: 18 }
            }
            span { class: "score-title", "{score().title}" }
            if let Some(error) = error() {
                span { class: "error", "{error}" }
            }
            ZoomControls {}
        }
    }
}

/// Zoom out, reset and zoom in buttons, at the far end of the toolbar
#[component]
fn ZoomControls() -> Element {
    let mut zoom = use_zoom();

    rsx! {
        div { class: "zoom-controls",
            button {
                class: "zoom-step",
                title: "Zoom out (⌘−)",
                aria_label: "Zoom out",
                disabled: !zoom.can_zoom_out(),
                onclick: move |_| zoom.zoom_out(),
                ZoomOut { size: 16 }
            }
            button {
                class: "zoom-reset",
                title: "Reset to 100% (⌘0)",
                onclick: move |_| zoom.reset(),
                "{zoom.percent()}%"
            }
            button {
                class: "zoom-step",
                title: "Zoom in (⌘=)",
                aria_label: "Zoom in",
                disabled: !zoom.can_zoom_in(),
                onclick: move |_| zoom.zoom_in(),
                ZoomIn { size: 16 }
            }
        }
    }
}

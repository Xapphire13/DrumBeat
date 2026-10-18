#![warn(clippy::pedantic)]

use dioxus::prelude::*;
use lucide_dioxus::{FolderOpen, Metronome, ZoomIn, ZoomOut};

use crate::ui::{
    keyboard_shortcuts::use_keyboard_shortcuts,
    metronome_control::MetronomeControl,
    playback_bar::format_time,
    practice_tool::PracticeTool,
    providers::{
        loop_editor::use_loop_editor_provider,
        playback_manager::{use_playback_manager, use_playback_manager_provider},
        practice::{use_practice, use_practice_provider},
        score_manager::{load_score, use_score_manager, use_score_manager_provider},
        view::{View, use_view, use_view_provider},
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
    use_practice_provider();
    let view = use_view_provider();
    use_keyboard_shortcuts();

    rsx! {
        document::Link { rel: "stylesheet", href: MAIN_CSS }

        Toolbar {}

        match view() {
            View::Score => rsx! {
                Score {}
            },
            View::Metronome => rsx! {
                PracticeTool {}
            },
        }
    }
}

#[component]
fn Toolbar() -> Element {
    let mut score = use_score_manager();
    let mut playback_manager = use_playback_manager();
    let mut error = use_signal(|| None::<String>);
    let view = use_view();

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
            ViewTabs {}
            if let Some(error) = error() {
                span { class: "error", "{error}" }
            }
            match view() {
                View::Score => rsx! {
                    div { class: "toolbar-end",
                        MetronomeControl {}
                        ZoomControls {}
                    }
                },
                View::Metronome => rsx! {
                    PracticeTimer {}
                },
            }
        }
    }
}

/// Switches between the score and the practice metronome. The score stops
/// playing on the way to the metronome, and the metronome on the way back
#[component]
fn ViewTabs() -> Element {
    let mut view = use_view();
    let mut playback_manager = use_playback_manager();
    let mut practice = use_practice();

    let mut show = move |next: View| {
        match next {
            View::Score => practice.stop(),
            View::Metronome => playback_manager.pause(),
        }
        view.set(next);
    };
    let tab_class = |tab: View| {
        if view() == tab {
            "view-tab selected"
        } else {
            "view-tab"
        }
    };

    rsx! {
        div { class: "view-tabs", role: "tablist",
            button {
                class: tab_class(View::Score),
                role: "tab",
                aria_selected: view() == View::Score,
                onclick: move |_| show(View::Score),
                "Score"
            }
            button {
                class: tab_class(View::Metronome),
                role: "tab",
                aria_selected: view() == View::Metronome,
                onclick: move |_| show(View::Metronome),
                Metronome { size: 14 }
                "Metronome"
            }
        }
    }
}

/// How long the practice metronome has run this session
#[component]
fn PracticeTimer() -> Element {
    let practice = use_practice();

    rsx! {
        span { class: "practice-timer", title: "Time practised with the metronome this session",
            "Practiced "
            span { class: "practice-timer-time",
                {format_time(f64::from(practice.practised_seconds()))}
            }
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

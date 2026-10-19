#![warn(clippy::pedantic)]

use dioxus::prelude::*;
use lucide_dioxus::{
    Ellipsis, FolderOpen, Metronome, SkipBack, SkipForward, SlidersHorizontal, X, ZoomIn, ZoomOut,
};

use crate::{
    platform,
    ui::{
        keyboard_shortcuts::use_keyboard_shortcuts,
        metronome_control::MetronomeControl,
        playback_bar::format_time,
        practice_tool::{PracticeTool, use_settings_sheet, use_settings_sheet_provider},
        providers::{
            loop_editor::use_loop_editor_provider,
            playback_manager::{use_playback_manager, use_playback_manager_provider},
            practice::{use_practice, use_practice_provider},
            score_manager::{load_score, use_score_manager, use_score_manager_provider},
            view::{View, use_view, use_view_provider},
            zoom::{use_zoom, use_zoom_provider},
        },
        score::Score,
    },
};

const MAIN_CSS: Asset = asset!("/assets/main.css");

#[component]
pub fn App() -> Element {
    use_playback_manager_provider();
    use_loop_editor_provider();
    use_score_manager_provider();
    use_zoom_provider();
    use_practice_provider();
    use_settings_sheet_provider();
    let view = use_view_provider();
    use_keyboard_shortcuts();
    let settings_sheet = use_settings_sheet();
    // The metronome's settings sheet is the playback bar's colour too
    use_effect(move || platform::color_safe_areas(view() == View::Score || settings_sheet()));

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

    let handle_open = move |_: MouseEvent| async move {
        let Some(path) = platform::pick_score().await else {
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
                        div { class: "regular-only",
                            MetronomeControl {}
                        }
                        ZoomControls {}
                        MoreMenu {}
                    }
                },
                View::Metronome => rsx! {
                    div { class: "toolbar-end",
                        PracticeTimer {}
                        SettingsButton {}
                    }
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

/// On a phone, where the toolbar has no room for the zoom controls or a
/// keyboard for the shortcuts: zoom, jumps to either end, and ending the loop
#[component]
fn MoreMenu() -> Element {
    let mut open = use_signal(|| false);
    let mut zoom = use_zoom();
    let mut playback_manager = use_playback_manager();
    let score = use_score_manager();
    let has_loop = playback_manager.loop_range().is_some();

    rsx! {
        div { class: "more-menu compact-only",
            button {
                class: "more-button",
                aria_label: "More",
                aria_haspopup: "menu",
                aria_expanded: open(),
                onclick: move |_| open.toggle(),
                Ellipsis { size: 20 }
            }
            if open() {
                div {
                    class: "toolbar-popover-backdrop",
                    onclick: move |_| open.set(false),
                }
                div { class: "more-popover", role: "menu",
                    div { class: "more-zoom",
                        button {
                            aria_label: "Zoom out",
                            disabled: !zoom.can_zoom_out(),
                            onclick: move |_| zoom.zoom_out(),
                            ZoomOut { size: 18 }
                        }
                        button { onclick: move |_| zoom.reset(),
                            span { class: "more-zoom-percent", "{zoom.percent()}%" }
                            span { class: "more-zoom-label", "Zoom" }
                        }
                        button {
                            aria_label: "Zoom in",
                            disabled: !zoom.can_zoom_in(),
                            onclick: move |_| zoom.zoom_in(),
                            ZoomIn { size: 18 }
                        }
                    }
                    button {
                        class: "more-item",
                        role: "menuitem",
                        onclick: move |_| {
                            playback_manager.skip_to_start(&score.peek());
                            open.set(false);
                        },
                        "Go to start"
                        SkipBack { size: 18 }
                    }
                    button {
                        class: "more-item",
                        role: "menuitem",
                        onclick: move |_| {
                            playback_manager.skip_to_end(&score.peek());
                            open.set(false);
                        },
                        "Go to end"
                        SkipForward { size: 18 }
                    }
                    button {
                        class: "more-item danger",
                        role: "menuitem",
                        disabled: !has_loop,
                        onclick: move |_| {
                            playback_manager.clear_loop();
                            open.set(false);
                        },
                        "Clear loop"
                        X { size: 18 }
                    }
                }
            }
        }
    }
}

/// Opens the metronome's settings as a sheet, on a phone
#[component]
fn SettingsButton() -> Element {
    let mut open = use_settings_sheet();

    rsx! {
        button {
            class: "more-button compact-only",
            aria_label: "Metronome settings",
            onclick: move |_| open.set(true),
            SlidersHorizontal { size: 20 }
        }
    }
}

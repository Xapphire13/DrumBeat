use dioxus::prelude::*;
use staff::Staff;

use crate::{
    data::{measure::Measure, score::Hairpin},
    platform,
    ui::{
        hooks::use_size,
        playback_bar::PlaybackBar,
        providers::{
            loop_editor::{LoopEditor, use_loop_editor},
            playback_manager::use_playback_manager,
            score_manager::use_score_manager,
            zoom::use_zoom,
        },
    },
};

mod beaming;
mod grace_note;
mod measure;
mod measure_scale;
mod note;
mod rest;
mod staff;

/// Distance between staff lines
const LINE_HEIGHT: f32 = 20.0;
/// How large the score is drawn at 100% zoom, relative to the sizes it's laid
/// out with
const BASE_SCALE: f32 = 0.675;
/// Space between the window edges and the staves' ends
const STAFF_MARGIN: f32 = 16.0;
/// Room at the start of each staff for its percussion clef, in line heights
const CLEF_ROOM: f32 = 2.0;

/// A staff's size in the units it's laid out in, which are pixels at 100%
/// zoom
#[derive(PartialEq, Clone)]
pub struct StaffDimensions {
    width: f32,
    height: f32,
    /// How much larger the staff is drawn than it's laid out
    zoom: f32,
}

/// The measures on one staff, with the width each one is drawn at
#[derive(PartialEq, Clone)]
pub struct StaffLayout {
    /// The number shown for the staff's first measure, counting from 1
    first_measure_number: usize,
    measures: Vec<Measure>,
    widths: Vec<f32>,
    /// Where the staff starts, leaving a margin before it
    start: f32,
    /// Where the first measure starts, after the clef
    left: f32,
    /// The hairpins that reach into this staff
    hairpins: Vec<Hairpin>,
}

#[component]
pub fn Score() -> Element {
    let (width, _, handle_on_resize) = use_size();
    let score = use_score_manager();
    let zoom = use_zoom();
    let zoom = zoom.factor() * BASE_SCALE;
    let mut loop_editor = use_loop_editor();
    let mut playback_manager = use_playback_manager();
    use_hover_tracking(loop_editor);
    let staves_class = if loop_editor.is_dragging() {
        "staves dragging"
    } else {
        "staves"
    };
    // The staves are laid out as if the window were `1 / zoom` times as wide,
    // then drawn `zoom` times larger, so zooming reflows the measures to fit
    // the window while everything in them, strokes included, scales as one
    let width = width as f32 / zoom;
    let staves = use_memo(use_reactive((&width, &zoom), move |(width, zoom)| {
        let score = score.read();
        let mut staves = layout_staves(&score.measures, width, STAFF_MARGIN / zoom);
        for staff in &mut staves {
            let (Some(first), Some(last)) = (staff.measures.first(), staff.measures.last()) else {
                continue;
            };
            let (start, end) = (first.timestamp, last.end_timestamp());
            staff.hairpins = score
                .hairpins
                .iter()
                .filter(|hairpin| hairpin.start < end && hairpin.end > start)
                .cloned()
                .collect();
        }
        staves
    }));

    rsx! {
        div {
            class: "score",
            onresize: handle_on_resize,
            // Drags over the score and the measure strip end wherever the
            // button is let go
            onmouseup: move |_| loop_editor.finish(&mut playback_manager, &score.peek()),

            div { class: staves_class,
                if score().measures.is_empty() {
                    div { class: "empty-score", "Open a Guitar Pro 8 file to get started" }
                }

                {staves().into_iter().map(|layout| rsx! {
                    Staff {
                        dimensions: StaffDimensions {
                            width,
                            height: staff::height(
                                LINE_HEIGHT,
                                !layout.hairpins.is_empty(),
                                staff::has_sections(&layout.measures),
                            ),
                            zoom,
                        },
                        layout,
                    }
                })}
            }

            PlaybackBar {}
        }
    }
}

/// Reports whether the pointer is anywhere over the staves. Done in the page
/// with `mouseover`, which names the element the pointer is now over: the
/// staves are made of a row per staff, and the leave and enter events as the
/// pointer crosses from one row to the next reach the staves too, so they
/// can't tell whether it's still inside
const HOVER_LISTENER: &str = r#"
let inside = false;
const update = (now) => {
    if (now !== inside) {
        inside = now;
        dioxus.send(now);
    }
};
document.addEventListener("mouseover", (event) => {
    const target = event.target;
    update(!!(target && target.closest && target.closest(".staves")));
});
document.documentElement.addEventListener("mouseleave", () => update(false));
"#;

fn use_hover_tracking(mut loop_editor: LoopEditor) {
    use_future(move || async move {
        if !platform::HAS_HOVER {
            return;
        }
        let mut listener = document::eval(HOVER_LISTENER);
        while let Ok(hovering) = listener.recv::<bool>().await {
            loop_editor.set_hovering_score(hovering);
        }
    });
}

/// Stretching a measure past this many times its natural width looks sparse,
/// so a short last staff is left short instead
const MAX_LAST_STAFF_STRETCH: f32 = 1.5;

/// Break the measures into staves that fit a window `window_width` wide, with
/// `margin` at each side, giving each measure room for its notes, then stretch
/// each staff's measures to fill it
fn layout_staves(measures: &[Measure], window_width: f32, margin: f32) -> Vec<StaffLayout> {
    let line_height = LINE_HEIGHT;
    let width = window_width - 2.0 * margin - CLEF_ROOM * line_height;
    if width <= 0.0 {
        return vec![];
    }

    let mut staves = vec![];
    let empty_staff = |first_measure_number| StaffLayout {
        first_measure_number,
        measures: vec![],
        widths: vec![],
        start: margin,
        left: margin + CLEF_ROOM * line_height,
        hairpins: vec![],
    };
    let mut current = empty_staff(1);
    let mut current_width = 0.0;

    for (i, measure) in measures.iter().enumerate() {
        let measure_width = measure_scale::natural_width(measure, line_height);
        if !current.measures.is_empty() && current_width + measure_width > width {
            staves.push(std::mem::replace(&mut current, empty_staff(i + 1)));
            current_width = 0.0;
        }
        current.measures.push(measure.clone());
        current.widths.push(measure_width);
        current_width += measure_width;
    }
    if !current.measures.is_empty() {
        staves.push(current);
    }

    let staff_count = staves.len();
    for (i, staff) in staves.iter_mut().enumerate() {
        let natural_total: f32 = staff.widths.iter().sum();
        let mut stretch = width / natural_total;
        if i + 1 == staff_count {
            stretch = stretch.min(MAX_LAST_STAFF_STRETCH);
        }
        for measure_width in &mut staff.widths {
            *measure_width *= stretch;
        }
    }

    staves
}

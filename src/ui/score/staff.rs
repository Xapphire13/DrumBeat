use dioxus::{html::geometry::euclid::Point2D, prelude::*};

use crate::{
    data::{
        app_mode::{AppMode, EditMode},
        measure::Measure as MeasureData,
        note::Note,
        position::Position,
    },
    ui::score::{measure::Measure, measure_scale::MeasureScale},
};

#[component]
pub fn Staff(
    width: f32,
    height: f32,
    measures: Vec<MeasureData>,
    handle_remove_note: Callback<Note>,
    handle_add_note: Callback<Note>,
) -> Element {
    let app_mode = use_context::<Signal<AppMode>>();
    let mut mouse_position = use_signal::<Option<Position>>(|| None);
    let mut preview_note: Signal<Option<Note>> = use_signal(|| None);

    let handle_on_mouse_move = move |ev: MouseEvent| {
        let Point2D { x, y, .. } = ev.coordinates().element();
        mouse_position.set(Some(Position {
            x: x as f32,
            y: y as f32,
        }));
    };

    let handle_on_mouse_leave = move |_: MouseEvent| {
        mouse_position.set(None);
        preview_note.set(None);
    };

    let handle_on_click = move |_: MouseEvent| {
        if let Some(note) = preview_note() {
            match app_mode() {
                AppMode::Edit {
                    edit_mode: EditMode::Add,
                    ..
                } => {
                    handle_add_note(note);
                    preview_note.set(None);
                }
                AppMode::Edit {
                    edit_mode: EditMode::Remove,
                    ..
                } => {
                    handle_remove_note(note);
                    preview_note.set(None);
                }
                _ => {}
            }
        }
    };

    let line_height = height / 8.;
    let num_measures = measures.len() as f32;

    rsx! {
        div {
            class: "staff-container",
            onmousemove: handle_on_mouse_move,
            onmouseleave: handle_on_mouse_leave,
            onclick: handle_on_click,
            style: "height: {height}px",

            svg { class: "staff", width: "{width}px", height: "{height}px",

                g {
                    for line_number in 1..=5 {
                        line {
                            x1: "0",
                            y1: "{(2. * line_height) + (line_number as f32)*line_height}",
                            x2: "{width}",
                            y2: "{(2. * line_height) + (line_number as f32)*line_height}",
                        }
                    }
                }

                {
                    measures
                        .into_iter()
                        .enumerate()
                        .map(|(i, measure)| {
                            let left_bound = (i as f32) * width / num_measures;
                            let right_bound = (i as f32 + 1.) * width / num_measures;
                            let padded_left_bound = left_bound + 1.5 * line_height;
                            let padded_right_bound = right_bound - 0.5 * line_height;
                            let scale = MeasureScale::new(
                                (padded_left_bound, padded_right_bound),
                                height - 1.5 * line_height,
                                line_height,
                                i as u32 * 256,
                            );
                            let mouse_position = 'mouse_position: {
                                if let Some(Position { x, y }) = mouse_position() {
                                    if x >= left_bound && x < right_bound {
                                        break 'mouse_position Some(Position {
                                            x: x - padded_left_bound,
                                            y,
                                        });
                                    }
                                }
                                None
                            };
                            rsx! {
                                // Draw vertical line between measures
                                if i > 0 {
                                    line {
                                        x1: "{left_bound}",
                                        y1: "{scale.get_y(-0.5)}",
                                        x2: "{left_bound}",
                                        y2: "{scale.get_y(3.5)}",
                                    }
                                }
                                Measure {
                                    scale,
                                    notes: measure.notes.clone(),
                                    mouse_position,
                                    on_preview_note_created: move |note| {
                                        preview_note.set(note);
                                    },
                                }
                            }
                        })
                }
            }
        }
    }
}

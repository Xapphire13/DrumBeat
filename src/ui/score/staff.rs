use dioxus::{html::geometry::euclid::Point2D, prelude::*};

use crate::{
    data::{
        measure::Measure as MeasureData,
        note::{Note, NoteValue, Surface},
        position::Position,
    },
    ui::score::{measure::Measure, measure_scale::MeasureScale},
};

#[component]
pub fn Staff() -> Element {
    let mut width = use_signal(|| 0i32);
    let mut height = use_signal(|| 0i32);
    let mut mouse_position = use_signal::<Option<Position>>(|| None);
    let mut preview_note: Signal<Option<Note>> = use_signal(|| None);
    let mut measures = use_signal(|| {
        vec![
            MeasureData {
                notes: vec![
                    Note {
                        value: NoteValue::Quarter,
                        surface: Surface::Kick,
                        timestamp: 0,
                    },
                    Note {
                        value: NoteValue::Quarter,
                        surface: Surface::Hihat,
                        timestamp: 0,
                    },
                    Note {
                        value: NoteValue::Eighth,
                        surface: Surface::Hihat,
                        timestamp: 32,
                    },
                    Note {
                        value: NoteValue::Quarter,
                        surface: Surface::Snare,
                        timestamp: 64,
                    },
                    Note {
                        value: NoteValue::Quarter,
                        surface: Surface::Hihat,
                        timestamp: 64,
                    },
                    Note {
                        value: NoteValue::Eighth,
                        surface: Surface::Hihat,
                        timestamp: 96,
                    },
                    Note {
                        value: NoteValue::Quarter,
                        surface: Surface::Kick,
                        timestamp: 128,
                    },
                    Note {
                        value: NoteValue::Quarter,
                        surface: Surface::Hihat,
                        timestamp: 128,
                    },
                    Note {
                        value: NoteValue::Eighth,
                        surface: Surface::Hihat,
                        timestamp: 160,
                    },
                    Note {
                        value: NoteValue::Quarter,
                        surface: Surface::Snare,
                        timestamp: 192,
                    },
                    Note {
                        value: NoteValue::Quarter,
                        surface: Surface::Hihat,
                        timestamp: 192,
                    },
                    Note {
                        value: NoteValue::Eighth,
                        surface: Surface::Hihat,
                        timestamp: 224,
                    },
                ],
                timestamp: 0,
            },
            MeasureData {
                timestamp: 256,
                notes: vec![
                    Note {
                        value: NoteValue::Quarter,
                        surface: Surface::Snare,
                        timestamp: 256,
                    },
                    Note {
                        value: NoteValue::Quarter,
                        surface: Surface::Snare,
                        timestamp: 320,
                    },
                    Note {
                        value: NoteValue::Quarter,
                        surface: Surface::Snare,
                        timestamp: 384,
                    },
                    Note {
                        value: NoteValue::Quarter,
                        surface: Surface::Snare,
                        timestamp: 448,
                    },
                ],
            },
        ]
    });

    let handle_on_mouse_move = move |ev: MouseEvent| {
        let Point2D { x, y, .. } = ev.coordinates().element();
        mouse_position.set(Some(Position {
            x: x as f32,
            y: y as f32,
        }));
    };

    let handle_on_mouse_leave = move |_: MouseEvent| {
        mouse_position.set(None);
    };

    let handle_on_resize = move |ev: ResizeEvent| {
        width.set(ev.get_border_box_size().unwrap().width as i32);
        height.set(ev.get_border_box_size().unwrap().height as i32);
    };

    let handle_on_click = move |_: MouseEvent| {
        if let Some(preview_note) = preview_note() {
            let updated_measures = measures()
                .into_iter()
                .map(|mut measure| {
                    if measure.timestamp <= preview_note.timestamp
                        && preview_note.timestamp - measure.timestamp < 256
                    {
                        measure.notes.push(preview_note.clone());
                    }
                    measure
                })
                .collect();

            measures.set(updated_measures);
        }
    };

    let line_height = height() as f32 / 8.;
    let num_measures = measures.len() as f32;

    rsx! {
        div {
            class: "staff-container",
            onresize: handle_on_resize,
            onmousemove: handle_on_mouse_move,
            onmouseleave: handle_on_mouse_leave,
            onclick: handle_on_click,
            svg { class: "staff", width: "{width}px", height: "{height}px",

                g {
                    line {
                        x1: "0",
                        y1: "{(2. * line_height) + line_height}",
                        x2: "{width}",
                        y2: "{(2. * line_height) + line_height}",
                    }

                    line {
                        x1: "0",
                        y1: "{(2. * line_height) + 2.*line_height}",
                        x2: "{width}",
                        y2: "{(2. * line_height) + 2.*line_height}",
                    }

                    line {
                        x1: "0",
                        y1: "{(2. * line_height) + 3.*line_height}",
                        x2: "{width}",
                        y2: "{(2. * line_height) + 3.*line_height}",
                    }

                    line {
                        x1: "0",
                        y1: "{(2. * line_height) + 4.*line_height}",
                        x2: "{width}",
                        y2: "{(2. * line_height) + 4.*line_height}",
                    }

                    line {
                        x1: "0",
                        y1: "{(2. * line_height) + 5.*line_height}",
                        x2: "{width}",
                        y2: "{(2. * line_height) + 5.*line_height}",
                    }
                }

                {
                    measures()
                        .into_iter()
                        .enumerate()
                        .map(|(i, measure)| {
                            let left_bound = (i as f32) * width() as f32 / num_measures;
                            let right_bound = (i as f32 + 1.) * width() as f32 / num_measures;
                            let padded_left_bound = left_bound + 1.5 * line_height;
                            let padded_right_bound = right_bound - 0.5 * line_height;
                            let scale = MeasureScale::new(
                                (padded_left_bound, padded_right_bound),
                                height() as f32 - 1.5 * line_height,
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
                                        preview_note.set(Some(note));
                                    },
                                }
                            }
                        })
                }
            }
        }
    }
}

use dioxus::prelude::*;

use crate::{
    data::note::{Note, NoteValue, Surface},
    ui::score::{measure::Measure, measure_scale::MeasureScale},
};

#[component]
pub fn Staff() -> Element {
    let mut width = use_signal(|| 0i32);
    let mut height = use_signal(|| 0i32);

    let handleOnResize = move |ev: ResizeEvent| {
        width.set(ev.get_border_box_size().unwrap().width as i32);
        height.set(ev.get_border_box_size().unwrap().height as i32);
    };

    let line_height = height() as f32 / 8.;

    let measures: [Vec<Note>; 2] = [
        vec![
            Note {
                value: NoteValue::Quarter,
                surface: Surface::Kick,
                offset: 0,
            },
            Note {
                value: NoteValue::Quarter,
                surface: Surface::Hihat,
                offset: 0,
            },
            Note {
                value: NoteValue::Eighth,
                surface: Surface::Hihat,
                offset: 1,
            },
            Note {
                value: NoteValue::Quarter,
                surface: Surface::Snare,
                offset: 1,
            },
            Note {
                value: NoteValue::Quarter,
                surface: Surface::Hihat,
                offset: 1,
            },
            Note {
                value: NoteValue::Eighth,
                surface: Surface::Hihat,
                offset: 3,
            },
            Note {
                value: NoteValue::Quarter,
                surface: Surface::Kick,
                offset: 2,
            },
            Note {
                value: NoteValue::Quarter,
                surface: Surface::Hihat,
                offset: 2,
            },
            Note {
                value: NoteValue::Eighth,
                surface: Surface::Hihat,
                offset: 5,
            },
            Note {
                value: NoteValue::Quarter,
                surface: Surface::Snare,
                offset: 3,
            },
            Note {
                value: NoteValue::Quarter,
                surface: Surface::Hihat,
                offset: 3,
            },
            Note {
                value: NoteValue::Eighth,
                surface: Surface::Hihat,
                offset: 7,
            },
        ],
        vec![
            Note {
                value: NoteValue::Quarter,
                surface: Surface::Snare,
                offset: 0,
            },
            Note {
                value: NoteValue::Quarter,
                surface: Surface::Snare,
                offset: 1,
            },
            Note {
                value: NoteValue::Quarter,
                surface: Surface::Snare,
                offset: 2,
            },
            Note {
                value: NoteValue::Quarter,
                surface: Surface::Snare,
                offset: 3,
            },
        ],
    ];
    let num_measures = measures.len() as f32;

    rsx! {
        div { class: "staff-container", onresize: handleOnResize,
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
                    measures
                        .into_iter()
                        .enumerate()
                        .map(|(i, notes)| {
                            let left_bound = (i as f32) * width() as f32 / num_measures;
                            let right_bound = (i as f32 + 1.) * width() as f32 / num_measures;
                            let scale = MeasureScale::new(
                                (left_bound + 1.5 * line_height, right_bound),
                                height() as f32 - 1.5 * line_height,
                                line_height,
                            );
                            rsx! {
                                if i > 0 {
                                    line {
                                        x1: "{left_bound}",
                                        y1: "{scale.get_y(-0.5)}",
                                        x2: "{left_bound}",
                                        y2: "{scale.get_y(3.5)}",
                                    }
                                }
                                Measure { scale, notes }
                            }
                        })
                }
            }
        }
    }
}

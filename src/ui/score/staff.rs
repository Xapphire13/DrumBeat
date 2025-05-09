use dioxus::{dioxus_core::SpawnIfAsync, prelude::*};

use crate::ui::score::note::{Note, NoteValue, Surface};

use super::{measure::Measure, measure_scale::MeasureScaleBuilder};

#[component]
pub fn Staff() -> Element {
    let mut width = use_signal(|| 0i32);
    let mut height = use_signal(|| 0i32);

    let handleOnResize = move |ev: ResizeEvent| {
        width.set(ev.get_border_box_size().unwrap().width as i32);
        height.set(ev.get_border_box_size().unwrap().height as i32);
    };

    let line_height = height() / 6;
    let mut scale_builder = MeasureScaleBuilder::default();
    scale_builder.height(height());

    rsx! {
        div { class: "staff-container", onresize: handleOnResize,
            svg { class: "staff", width: "{width}px", height: "{height}px",

                g {
                    line {
                        x1: "0",
                        y1: "{line_height}",
                        x2: "{width}",
                        y2: "{line_height}",
                    }

                    line {
                        x1: "0",
                        y1: "{2*line_height}",
                        x2: "{width}",
                        y2: "{2*line_height}",
                    }

                    line {
                        x1: "0",
                        y1: "{3*line_height}",
                        x2: "{width}",
                        y2: "{3*line_height}",
                    }

                    line {
                        x1: "0",
                        y1: "{4*line_height}",
                        x2: "{width}",
                        y2: "{4*line_height}",
                    }

                    line {
                        x1: "0",
                        y1: "{5*line_height}",
                        x2: "{width}",
                        y2: "{5*line_height}",
                    }
                }

                Measure {
                    scale: scale_builder.clone().x_range((0, width() / 2)).build()?,
                    notes: vec![
                        Note {
                            value: NoteValue::Quarter,
                            surface: Surface::Snare,
                            offset: 0,
                        },
                        Note {
                            value: NoteValue::Eighth,
                            surface: Surface::Snare,
                            offset: 1,
                        },
                        Note {
                            value: NoteValue::Sixteenth,
                            surface: Surface::Snare,
                            offset: 3,
                        },
                        Note {
                            value: NoteValue::Thirtysecondth,
                            surface: Surface::Snare,
                            offset: 7,
                        },
                        Note {
                            value: NoteValue::Quarter,
                            surface: Surface::Snare,
                            offset: 1,
                        },
                        Note {
                            value: NoteValue::Quarter,
                            surface: Surface::Snare,
                            offset: 3,
                        },
                    ],
                }
                Measure {
                    scale: scale_builder.clone().x_range((width() / 2, width())).build()?,
                    notes: vec![
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
                }
            }
        }
    }
}

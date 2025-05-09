use dioxus::prelude::*;

use crate::ui::score::measure::Measure;

const HEIGHT: i16 = 100;

#[component]
pub fn Staff() -> Element {
    let mut width = use_signal(|| 0i32);

    rsx! {
        div { class: "staff", onmounted: move |element| async move {
            width.set(element.get_client_rect().await.unwrap().width() as i32);
        },
            svg { width: "{width}px", height: "{HEIGHT}px",

                line {
                    x1: "0",
                    y1: "{HEIGHT/6}",
                    x2: "{width}",
                    y2: "{HEIGHT/6}",
                    stroke: "black",
                }

                line {
                    x1: "0",
                    y1: "{2*HEIGHT/6}",
                    x2: "{width}",
                    y2: "{2*HEIGHT/6}",
                    stroke: "black",
                }

                line {
                    x1: "0",
                    y1: "{3*HEIGHT/6}",
                    x2: "{width}",
                    y2: "{3*HEIGHT/6}",
                    stroke: "black",
                }

                line {
                    x1: "0",
                    y1: "{4*HEIGHT/6}",
                    x2: "{width}",
                    y2: "{4*HEIGHT/6}",
                    stroke: "black",
                }

                line {
                    x1: "0",
                    y1: "{5*HEIGHT/6}",
                    x2: "{width}",
                    y2: "{5*HEIGHT/6}",
                    stroke: "black",
                }
            }

            Measure {}
            Measure {}
            Measure {}
            Measure {}
        }
    }
}

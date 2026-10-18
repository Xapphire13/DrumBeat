use dioxus::prelude::*;

/// The five staff lines, which sit on line numbers -0.5 to 3.5
#[component]
pub fn StaffLines(line_height: f32, baseline: f32, width: f32) -> Element {
    rsx! {
        g {
            for line_number in [-0.5f32, 0.5, 1.5, 2.5, 3.5] {
                line {
                    x1: "0",
                    y1: "{baseline - line_number * line_height}",
                    x2: "{width}",
                    y2: "{baseline - line_number * line_height}",
                }
            }
        }
    }
}

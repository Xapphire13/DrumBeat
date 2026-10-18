use dioxus::prelude::*;

/// The five staff lines, which sit on line numbers -0.5 to 3.5
#[component]
pub fn StaffLines(line_height: f32, baseline: f32, x_bounds: (f32, f32)) -> Element {
    rsx! {
        g {
            for line_number in [-0.5f32, 0.5, 1.5, 2.5, 3.5] {
                line {
                    x1: "{x_bounds.0}",
                    y1: "{baseline - line_number * line_height}",
                    x2: "{x_bounds.1}",
                    y2: "{baseline - line_number * line_height}",
                }
            }
        }
    }
}

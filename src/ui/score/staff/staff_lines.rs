use dioxus::prelude::*;

use crate::ui::score::StaffDimensions;

#[component]
pub fn StaffLines(
    line_height: f32,
    number_of_lines: u8,
    staff_dimensions: StaffDimensions,
) -> Element {
    let StaffDimensions { width, .. } = staff_dimensions;

    rsx! {
        g {
            for line_number in 1..=number_of_lines {
                line {
                    x1: "0",
                    y1: "{(2. * line_height) + (line_number as f32)*line_height}",
                    x2: "{width}",
                    y2: "{(2. * line_height) + (line_number as f32)*line_height}",
                }
            }
        }
    }
}

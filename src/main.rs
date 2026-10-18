use dioxus_desktop::{Config, LogicalSize, WindowBuilder};
use ui::app::App;

mod data;
// The loader reads more of the file than the app uses yet
#[allow(dead_code)]
mod gp;
mod ui;

fn main() {
    dioxus::LaunchBuilder::desktop()
        .with_cfg(
            Config::new().with_window(
                WindowBuilder::new()
                    .with_title("DrumBeat")
                    // Narrow enough to fit the playback bar's controls on one row
                    .with_min_inner_size(LogicalSize::new(880.0, 480.0)),
            ),
        )
        .launch(App);
}

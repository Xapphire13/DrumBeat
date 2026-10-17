use dioxus_desktop::{Config, WindowBuilder};
use ui::app::App;

mod data;
// The loader reads more of the file than the app uses yet
#[allow(dead_code)]
mod gp;
mod ui;

fn main() {
    dioxus::LaunchBuilder::desktop()
        .with_cfg(Config::new().with_window(WindowBuilder::new().with_title("DrumBeat")))
        .launch(App);
}

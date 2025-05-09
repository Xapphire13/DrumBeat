use dioxus_desktop::{Config, WindowBuilder};
use ui::app::App;

mod ui;

fn main() {
    dioxus::LaunchBuilder::desktop()
        .with_cfg(Config::new().with_window(WindowBuilder::new().with_title("DrumBeat")))
        .launch(App);
}

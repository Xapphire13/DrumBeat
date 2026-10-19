use ui::app::App;

mod audio;
mod data;
// The loader reads more of the file than the app uses yet
#[allow(dead_code)]
mod gp;
mod platform;
mod settings;
mod ui;

fn main() {
    #[cfg(feature = "desktop")]
    {
        use dioxus_desktop::{Config, LogicalSize, WindowBuilder};

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

    // Pinching zooms the score instead of the page
    #[cfg(not(feature = "desktop"))]
    dioxus::LaunchBuilder::mobile()
        .with_cfg(dioxus_desktop::Config::new().with_custom_index(
            r#"<!DOCTYPE html>
<html>
    <head>
        <title>DrumBeat</title>
        <meta name="viewport" content="width=device-width, initial-scale=1, maximum-scale=1, user-scalable=no">
    </head>
    <body>
        <div id="main"></div>
    </body>
</html>"#
                .to_string(),
        ))
        .launch(App);
}

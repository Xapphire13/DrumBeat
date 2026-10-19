//! What differs between the desktop and iOS: how a score file is picked, and
//! the audio session iOS needs before it plays anything

use std::path::PathBuf;

/// Whether a pointer can hover. A tap on iOS also sends the events for
/// hovering, and a page that changes as they arrive has the tap swallowed,
/// so there hover-only touches are left out
pub const HAS_HOVER: bool = !cfg!(target_os = "ios");

#[cfg(target_os = "ios")]
mod ios;

/// Asks for a Guitar Pro file to open, or `None` if it's dismissed
pub async fn pick_score() -> Option<PathBuf> {
    #[cfg(target_os = "ios")]
    {
        ios::pick_score().await
    }
    #[cfg(not(target_os = "ios"))]
    {
        rfd::FileDialog::new()
            .add_filter("Guitar Pro 8", &["gp"])
            .pick_file()
    }
}

/// Sets the app up to play sound. On iOS that means asking to be heard with
/// the ring switch on silent, as an instrument's practice tool should be
pub fn prepare_audio() {
    #[cfg(target_os = "ios")]
    ios::prepare_audio();
}

/// Colours the strips iOS keeps clear of the notch and home indicator to
/// match what borders them: the toolbar above, and below either the playback
/// bar's colour or the page's
pub fn color_safe_areas(playback_bar_below: bool) {
    #[cfg(target_os = "ios")]
    ios::color_safe_areas(playback_bar_below);
    #[cfg(not(target_os = "ios"))]
    let _ = playback_bar_below;
}

/// Flashes the strip under the home indicator along with the metronome's
/// stage, which can't reach it
pub fn flash_bottom_safe_area() {
    #[cfg(target_os = "ios")]
    ios::flash_bottom_safe_area();
}

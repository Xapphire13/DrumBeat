use dioxus::prelude::*;

/// What fills the window under the title bar
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum View {
    Score,
    /// The standalone metronome, for practising away from the score
    Metronome,
}

pub fn use_view_provider() -> Signal<View> {
    use_context_provider(|| Signal::new(View::Score))
}

pub fn use_view() -> Signal<View> {
    use_context::<Signal<View>>()
}

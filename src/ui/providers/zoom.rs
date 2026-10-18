use dioxus::prelude::*;

/// The zoom levels, in percent, stepped through by zooming in and out
const STEPS: [u32; 10] = [50, 67, 80, 90, 100, 110, 125, 150, 175, 200];
const DEFAULT_STEP: usize = 4;

/// How large the score is drawn. It's a view setting, kept only while the
/// app is open
#[derive(Clone, Copy)]
pub struct Zoom {
    step: Signal<usize>,
}

impl Zoom {
    pub fn percent(&self) -> u32 {
        STEPS[(self.step)()]
    }

    /// The size the score is drawn at, relative to 100%
    pub fn factor(&self) -> f32 {
        self.percent() as f32 / 100.0
    }

    pub fn can_zoom_in(&self) -> bool {
        (self.step)() + 1 < STEPS.len()
    }

    pub fn can_zoom_out(&self) -> bool {
        (self.step)() > 0
    }

    pub fn zoom_in(&mut self) {
        if self.can_zoom_in() {
            self.step += 1;
        }
    }

    pub fn zoom_out(&mut self) {
        if self.can_zoom_out() {
            self.step -= 1;
        }
    }

    pub fn reset(&mut self) {
        self.step.set(DEFAULT_STEP);
    }
}

pub fn use_zoom_provider() -> Zoom {
    use_context_provider(|| Zoom {
        step: Signal::new(DEFAULT_STEP),
    })
}

pub fn use_zoom() -> Zoom {
    use_context::<Zoom>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_step_is_100_percent() {
        assert_eq!(STEPS[DEFAULT_STEP], 100);
    }
}

use dioxus::prelude::*;

use crate::{
    data::{
        loop_range::{LoopPoint, LoopRange},
        score::Score,
    },
    ui::providers::playback_manager::PlaybackManager,
};

/// How far, in window pixels, the pointer goes before a press on the score
/// becomes a drag
const DRAG_THRESHOLD: f64 = 4.0;

/// A drag that's setting or changing the loop
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum LoopDrag {
    /// Across the score, or by one of the loop's tabs (`by_tab`). `anchor`
    /// is the end that stays put, written at `anchor_written`, and `current`
    /// the one under the pointer. `pressed_at` is where the button went down,
    /// in window pixels
    Score {
        anchor: LoopPoint,
        anchor_written: u32,
        current: LoopPoint,
        moved: bool,
        by_tab: bool,
        pressed_at: (f64, f64),
    },
    /// Across the measure strip, by index into the played measures
    Measures {
        anchor: usize,
        current: usize,
        moved: bool,
    },
}

impl LoopDrag {
    /// The loop the drag sets, once it has moved
    fn preview(self, score: &Score) -> Option<LoopRange> {
        let performance = &score.performance;
        match self {
            LoopDrag::Score {
                anchor,
                current,
                moved: true,
                ..
            } => Some(LoopRange::between(
                anchor,
                current,
                performance,
                &score.measures,
            )),
            LoopDrag::Measures {
                anchor,
                current,
                moved: true,
            } => LoopRange::between_measures(anchor, current, performance, &score.measures),
            _ => None,
        }
    }
}

/// What's going on with the loop in the UI: any drag in progress, and
/// whether the pointer is over the score, which shows the loop's tabs
#[derive(Clone, Copy)]
pub struct LoopEditor {
    drag: Signal<Option<LoopDrag>>,
    hovering_score: Signal<bool>,
}

impl LoopEditor {
    pub fn drag(&self) -> Option<LoopDrag> {
        (self.drag)()
    }

    pub fn is_dragging(&self) -> bool {
        self.drag.read().is_some()
    }

    pub fn start_drag(&mut self, drag: LoopDrag) {
        self.drag.set(Some(drag));
    }

    /// Move the dragged end of the loop to `point`, with the pointer at `at`
    /// in window pixels. Until the pointer has gone a few pixels from where
    /// it was pressed, it's still a click, so a wobble while clicking a note
    /// moves the playhead rather than looping the note
    pub fn drag_to(&mut self, point: LoopPoint, at: (f64, f64)) {
        if let Some(LoopDrag::Score {
            anchor,
            anchor_written,
            current,
            moved,
            by_tab,
            pressed_at,
        }) = self.drag.cloned()
            && current != point
            && (moved || (at.0 - pressed_at.0).hypot(at.1 - pressed_at.1) >= DRAG_THRESHOLD)
        {
            self.drag.set(Some(LoopDrag::Score {
                anchor,
                anchor_written,
                current: point,
                moved: true,
                by_tab,
                pressed_at,
            }));
        }
    }

    /// Move the dragged end of a measure loop to the played measure at `index`
    pub fn drag_to_measure(&mut self, index: usize) {
        if let Some(LoopDrag::Measures {
            anchor, current, ..
        }) = self.drag.cloned()
            && current != index
        {
            self.drag.set(Some(LoopDrag::Measures {
                anchor,
                current: index,
                moved: true,
            }));
        }
    }

    /// The loop the drag would set if released now, once it has moved
    pub fn preview(&self, score: &Score) -> Option<LoopRange> {
        self.drag()?.preview(score)
    }

    /// The loop to draw: the one being dragged out, or else the one playing
    pub fn shown(&self, playback_manager: &PlaybackManager, score: &Score) -> Option<LoopRange> {
        self.preview(score)
            .or_else(|| playback_manager.loop_range())
    }

    /// End the drag. A drag that moved sets the loop; one that didn't is a
    /// click, which moves the playhead to the note or measure clicked
    pub fn finish(&mut self, playback_manager: &mut PlaybackManager, score: &Score) {
        let Some(drag) = self.drag.take() else {
            return;
        };
        match (drag, drag.preview(score)) {
            (LoopDrag::Score { .. }, Some(range)) => playback_manager.set_loop(range, score, true),
            (LoopDrag::Measures { .. }, Some(range)) => {
                playback_manager.set_loop(range, score, false);
            }
            (LoopDrag::Score { anchor, .. }, None) => {
                if anchor.timestamp() < score.performance.end_timestamp() {
                    playback_manager.seek_on_score(anchor.timestamp(), score);
                }
            }
            (LoopDrag::Measures { anchor, .. }, None) => {
                if let Some(played) = score.performance.measures().get(anchor) {
                    playback_manager.seek(played.timestamp, score);
                }
            }
        }
    }

    pub fn is_hovering_score(&self) -> bool {
        (self.hovering_score)()
    }

    pub fn set_hovering_score(&mut self, hovering: bool) {
        if *self.hovering_score.peek() != hovering {
            self.hovering_score.set(hovering);
        }
    }
}

pub fn use_loop_editor_provider() -> LoopEditor {
    use_context_provider(|| LoopEditor {
        drag: Signal::new(None),
        hovering_score: Signal::new(false),
    })
}

pub fn use_loop_editor() -> LoopEditor {
    use_context::<LoopEditor>()
}

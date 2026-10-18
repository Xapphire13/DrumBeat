use dioxus::prelude::*;

use crate::{
    data::{loop_range::LoopRange, score::Score},
    ui::providers::playback_manager::PlaybackManager,
};

/// A drag that's setting or changing the loop
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum LoopDrag {
    /// Across notes in the score, or by one of the loop's tabs. `anchor` is
    /// the note that stays put and `current` the one under the pointer, both
    /// on the performance timeline
    Notes {
        anchor: u32,
        current: u32,
        moved: bool,
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
            LoopDrag::Notes {
                anchor,
                current,
                moved: true,
            } => Some(LoopRange::between_notes(
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

    /// Move the dragged end of the loop to the note at `timestamp`
    pub fn drag_to_note(&mut self, timestamp: u32) {
        if let Some(LoopDrag::Notes {
            anchor, current, ..
        }) = self.drag.cloned()
            && current != timestamp
        {
            self.drag.set(Some(LoopDrag::Notes {
                anchor,
                current: timestamp,
                moved: true,
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
            (_, Some(range)) => playback_manager.set_loop(range, score),
            (LoopDrag::Notes { anchor, .. }, None) => playback_manager.seek(anchor, score),
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

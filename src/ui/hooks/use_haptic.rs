use std::rc::Rc;

use dioxus::{hooks::use_root_context, prelude::use_hook};
use trackpad_haptic::FeedbackManager;

#[derive(Clone)]
pub struct Haptic {
    feedback_manager: Option<Rc<FeedbackManager>>,
}

impl Haptic {
    pub fn trigger(&self) {
        if let Some(feedback_manager) = &self.feedback_manager {
            feedback_manager.trigger();
        }
    }
}

pub fn use_haptic() -> Haptic {
    let feedback_manager = use_root_context(|| {
        if cfg!(target_os = "macos") {
            Some(Rc::new(FeedbackManager::default()))
        } else {
            None
        }
    });

    use_hook(move || Haptic { feedback_manager })
}

// The picker's file-type initialiser is deprecated in favour of one taking
// UTTypes, which would need another framework for the same result
#![allow(deprecated)]

use std::{cell::RefCell, path::PathBuf};

use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained,
    runtime::ProtocolObject,
};
use objc2_avf_audio::{AVAudioSession, AVAudioSessionCategoryPlayback};
use objc2_foundation::{NSArray, NSObject, NSObjectProtocol, NSString, NSURL};
use objc2_ui_kit::{
    UIApplication, UIDocumentPickerDelegate, UIDocumentPickerMode, UIDocumentPickerViewController,
    UIViewController,
};
use tokio::sync::oneshot;

/// Any file: iOS doesn't know Guitar Pro's type, so a wrong pick is reported
/// when it fails to load
const ANY_FILE: &str = "public.data";

struct Ivars {
    reply: RefCell<Option<oneshot::Sender<Option<PathBuf>>>>,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and the delegate
    // doesn't implement Drop
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Ivars]
    struct PickerDelegate;

    unsafe impl NSObjectProtocol for PickerDelegate {}

    unsafe impl UIDocumentPickerDelegate for PickerDelegate {
        #[unsafe(method(documentPicker:didPickDocumentsAtURLs:))]
        fn did_pick(&self, _picker: &UIDocumentPickerViewController, urls: &NSArray<NSURL>) {
            let path = urls
                .firstObject()
                .and_then(|url| url.path())
                .map(|path| PathBuf::from(path.to_string()));
            self.reply(path);
        }

        #[unsafe(method(documentPickerWasCancelled:))]
        fn was_cancelled(&self, _picker: &UIDocumentPickerViewController) {
            self.reply(None);
        }
    }
);

impl PickerDelegate {
    fn new(mtm: MainThreadMarker, reply: oneshot::Sender<Option<PathBuf>>) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(Ivars {
            reply: RefCell::new(Some(reply)),
        });
        unsafe { msg_send![super(this), init] }
    }

    fn reply(&self, path: Option<PathBuf>) {
        if let Some(reply) = self.ivars().reply.take() {
            let _ = reply.send(path);
        }
    }
}

/// Shows the Files picker over whatever is on screen. The pick is a copy in
/// the app's own storage, so it can be read without asking for access
pub async fn pick_score() -> Option<PathBuf> {
    // Dioxus runs event handlers on the main thread
    let mtm = MainThreadMarker::new()?;
    let presenter = top_view_controller(mtm)?;
    let (reply, picked) = oneshot::channel();
    // The picker holds its delegate weakly, so it's kept here until it replies
    let delegate = PickerDelegate::new(mtm, reply);

    let types = NSArray::from_retained_slice(&[NSString::from_str(ANY_FILE)]);
    let picker = unsafe {
        UIDocumentPickerViewController::initWithDocumentTypes_inMode(
            UIDocumentPickerViewController::alloc(mtm),
            &types,
            UIDocumentPickerMode::Import,
        )
    };
    unsafe {
        picker.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        presenter.presentViewController_animated_completion(&picker, true, None);
    }

    let path = picked.await.ok().flatten();
    drop(delegate);
    path
}

/// The view controller showing on top, to present the picker from
fn top_view_controller(mtm: MainThreadMarker) -> Option<Retained<UIViewController>> {
    let app = UIApplication::sharedApplication(mtm);
    let window = unsafe { app.keyWindow() }.or_else(|| app.windows().firstObject())?;
    let mut top = window.rootViewController()?;
    while let Some(presented) = unsafe { top.presentedViewController() } {
        top = presented;
    }
    Some(top)
}

pub fn prepare_audio() {
    unsafe {
        let session = AVAudioSession::sharedInstance();
        if let Some(playback) = AVAudioSessionCategoryPlayback {
            let _ = session.setCategory_error(playback);
        }
        let _ = session.setActive_error(true);
    }
}

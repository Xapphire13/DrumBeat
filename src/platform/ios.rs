// The picker's file-type initialiser is deprecated in favour of one taking
// UTTypes, which would need another framework for the same result
#![allow(deprecated)]

use std::{cell::RefCell, path::PathBuf};

use objc2::{
    DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send,
    rc::Retained,
    runtime::{AnyClass, ProtocolObject},
};
use objc2_avf_audio::{AVAudioSession, AVAudioSessionCategoryPlayback};
use objc2_core_foundation::{CGFloat, CGPoint, CGRect, CGSize};
use objc2_foundation::{NSArray, NSObject, NSObjectProtocol, NSString, NSURL};
use objc2_ui_kit::{
    UIApplication, UIColor, UIDocumentPickerDelegate, UIDocumentPickerMode,
    UIDocumentPickerViewController, UIView, UIViewAutoresizing, UIViewController,
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

/// The toolbar's and the playback bar's backgrounds, as in main.css
const TOOLBAR: u32 = 0x0f1116;
const PLAYBACK_BAR: u32 = 0x161922;

/// The web view lets what's behind it show through beyond the page: the
/// window in the toolbar's colour, with a strip in the playback bar's along
/// the bottom
pub fn color_safe_areas() {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let Some(web_view_class) = AnyClass::get(c"WKWebView") else {
        return;
    };
    for window in UIApplication::sharedApplication(mtm).windows() {
        window.setBackgroundColor(Some(&color(TOOLBAR)));
        let Some(web_view) = find_view(&window, web_view_class) else {
            continue;
        };
        let Some(container) = web_view.superview() else {
            continue;
        };
        unsafe {
            let clear = UIColor::clearColor();
            web_view.setOpaque(false);
            web_view.setBackgroundColor(Some(&clear));
            let _: () = msg_send![&*web_view, setUnderPageBackgroundColor: &*clear];
        }

        let bounds = container.bounds();
        let inset = container.safeAreaInsets().bottom;
        let strip = UIView::initWithFrame(
            UIView::alloc(mtm),
            CGRect::new(
                CGPoint::new(0.0, bounds.size.height - inset),
                CGSize::new(bounds.size.width, inset),
            ),
        );
        strip.setBackgroundColor(Some(&color(PLAYBACK_BAR)));
        unsafe {
            strip.setAutoresizingMask(
                UIViewAutoresizing::FlexibleWidth | UIViewAutoresizing::FlexibleTopMargin,
            );
            container.insertSubview_belowSubview(&strip, &web_view);
        }
    }
}

fn find_view(view: &UIView, class: &AnyClass) -> Option<Retained<UIView>> {
    view.subviews().into_iter().find_map(|subview| {
        if subview.isKindOfClass(class) {
            Some(subview)
        } else {
            find_view(&subview, class)
        }
    })
}

fn color(rgb: u32) -> Retained<UIColor> {
    let channel = |shift: u32| CGFloat::from(((rgb >> shift) & 0xff) as u8) / 255.0;
    unsafe { UIColor::colorWithRed_green_blue_alpha(channel(16), channel(8), channel(0), 1.0) }
}

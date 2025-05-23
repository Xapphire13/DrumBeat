use dioxus::prelude::*;

pub fn use_size() -> (f64, f64, EventHandler<ResizeEvent>) {
    let mut height = use_signal(|| 0.0);
    let mut width = use_signal(|| 0.0);

    let handle_on_resize = use_callback(move |ev: ResizeEvent| {
        width.set(ev.get_border_box_size().unwrap().width);
        height.set(ev.get_border_box_size().unwrap().height);
    });

    (width(), height(), handle_on_resize)
}

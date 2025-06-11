use dioxus::{html::geometry::euclid::Point2D, prelude::*};

use crate::data::position::Position;

pub struct MousePositionData {
    pub mouse_position: Option<Position>,
    pub handle_on_mouse_move: Callback<MouseEvent>,
    pub handle_on_mouse_leave: Callback<MouseEvent>,
}

pub fn use_mouse_position() -> MousePositionData {
    let mut mouse_position = use_signal::<Option<Position>>(|| None);

    let handle_on_mouse_move = Callback::new(move |ev: MouseEvent| {
        let Point2D { x, y, .. } = ev.coordinates().element();
        mouse_position.set(Some(Position {
            x: x as f32,
            y: y as f32,
        }));
    });

    let handle_on_mouse_leave = Callback::new(move |_: MouseEvent| {
        mouse_position.set(None);
    });

    MousePositionData {
        mouse_position: mouse_position(),
        handle_on_mouse_move,
        handle_on_mouse_leave,
    }
}

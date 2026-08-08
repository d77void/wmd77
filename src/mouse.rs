use penrose::builtin::actions::floating::{MouseDragHandler, MouseResizeHandler, sink_focused};
use penrose::core::bindings::click_handler;
use penrose::core::bindings::{MouseEventHandler, MouseState};
use penrose::map;
use penrose::x11rb::RustConn;
use std::collections::HashMap;

pub fn mouse_bindings() -> HashMap<MouseState, Box<dyn MouseEventHandler<RustConn>>> {
    use penrose::core::bindings::{
        ModifierKey::{Meta, Shift},
        MouseButton::{Left, Middle, Right},
    };

    map! {
        map_keys: |(button, modifiers)| MouseState { button, modifiers };

        (Left, vec![Shift, Meta]) => MouseDragHandler::boxed_default(),
        (Right, vec![Shift, Meta]) => MouseResizeHandler::boxed_default(),
        (Middle, vec![Shift, Meta]) => click_handler(sink_focused()),
    }
}

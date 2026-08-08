use penrose::extensions::hooks::{
    NamedScratchPad, ToggleNamedScratchPad, manage::FloatingCentered,
};
use penrose::x::query::ClassName;
use penrose::x11rb::RustConn;

pub fn create_terminal_scratchpad() -> (NamedScratchPad<RustConn>, ToggleNamedScratchPad) {
    NamedScratchPad::new(
        "terminal",
        "st -c StScratchpad",
        ClassName("StScratchpad"),
        FloatingCentered::new(0.8, 0.8),
        true,
    )
}

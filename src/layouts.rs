use penrose::builtin::layout::transformers::{Gaps, ReflectHorizontal, ReserveTop};
use penrose::builtin::layout::{Grid, MainAndStack, Monocle};
use penrose::core::layout::LayoutStack;
use penrose::extensions::layout::{Fibonacci, Tatami};
use penrose::stack;

const MAX_MAIN: u32 = 1;
const RATIO: f32 = 0.6;
const RATIO_STEP: f32 = 0.1;
const OUTER_PX: u32 = 5;
const INNER_PX: u32 = 5;
const TOP_PX: u32 = 18;

pub fn build_layouts() -> LayoutStack {
    stack!(
        MainAndStack::side(MAX_MAIN, RATIO, RATIO_STEP),
        ReflectHorizontal::wrap(MainAndStack::side(MAX_MAIN, RATIO, RATIO_STEP)),
        MainAndStack::bottom(MAX_MAIN, RATIO, RATIO_STEP),
        Monocle::boxed(),
        Grid::boxed(),
        Fibonacci::boxed_default(),
        Tatami::boxed_default()
    )
    .map(|layout| ReserveTop::wrap(Gaps::wrap(layout, OUTER_PX, INNER_PX), TOP_PX))
}

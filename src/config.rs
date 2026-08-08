use crate::{hooks, layouts, theme};
use penrose::core::Config;
use penrose::x11rb::RustConn;

pub fn build_config() -> Config<RustConn> {
    use penrose::extensions::hooks::add_ewmh_hooks;

    add_ewmh_hooks(Config {
        focused_border: theme::LAVENDER.into(),
        normal_border: theme::GREY.into(),
        default_layouts: layouts::build_layouts(),
        startup_hook: Some(hooks::startup_hook()),
        ..Config::default()
    })
}

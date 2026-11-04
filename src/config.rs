use crate::user_config::UserConfig;
use crate::{hooks, layouts, reload};
use penrose::core::Config;
use penrose::core::hooks::ManageHook;
use penrose::extensions::hooks::add_ewmh_hooks;
use penrose::extensions::hooks::manage::{FloatingCentered, SetWorkspace};
use penrose::x::query::ClassName;
use penrose::x11rb::RustConn;

pub fn build_config(user: &UserConfig) -> Config<RustConn> {
    let mut config = add_ewmh_hooks(Config {
        focused_border: user.theme.accent.into(),
        normal_border: user.theme.grey.into(),
        default_layouts: layouts::build_layouts(user.bar.reserve_top),
        manage_hook: manage_hook(user),
        startup_hook: Some(hooks::startup_hook(&user.apps.startup_script)),
        ..Config::default()
    });

    // SIGHUP re-reads config.toml (see reload.rs)
    config.compose_or_set_event_hook(reload::event_hook);
    config
}

/// `[[window_rule]]` (class -> workspace) and `[[float_rule]]` (class floats
/// centered) from config.toml. ClassName/SetWorkspace need &'static str; the
/// rules are loaded once per process, so leaking them is fine.
fn manage_hook(user: &UserConfig) -> Option<Box<dyn ManageHook<RustConn>>> {
    let leak = |s: &str| -> &'static str { Box::leak(s.to_owned().into_boxed_str()) };

    let mut hooks: Vec<Box<dyn ManageHook<RustConn>>> = user
        .window_rules
        .iter()
        .map(|r| ManageHook::boxed((ClassName(leak(&r.class)), SetWorkspace(leak(&r.workspace)))))
        .collect();

    hooks.extend(user.float_rules.iter().map(|r| {
        ManageHook::boxed((
            ClassName(leak(&r.class)),
            FloatingCentered::new(r.width_ratio, r.height_ratio),
        ))
    }));

    if hooks.is_empty() {
        None
    } else {
        Some(Box::new(hooks))
    }
}

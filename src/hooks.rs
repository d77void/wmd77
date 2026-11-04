use penrose::extensions::hooks::SpawnOnStartup;
use penrose::x11rb::RustConn;

/// Runs `[apps] startup_script` once at startup. Unlike pwm, no theme name is
/// passed: wmd77 has no per-theme wallpapers.
pub fn startup_hook(script: &str) -> Box<dyn penrose::core::hooks::StateHook<RustConn>> {
    SpawnOnStartup::boxed(script.to_owned())
}

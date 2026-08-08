use penrose::extensions::hooks::SpawnOnStartup;
use penrose::x11rb::RustConn;

pub fn startup_hook() -> Box<dyn penrose::core::hooks::StateHook<RustConn>> {
    SpawnOnStartup::boxed("/etc/xdg/wmd77/startup.sh")
}

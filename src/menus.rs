use penrose::builtin::actions::{exit, key_handler};
use penrose::core::bindings::KeyEventHandler;
use penrose::extensions::util::dmenu::{DMenu, DMenuConfig, MenuMatch};
use penrose::util::spawn;
use penrose::x11rb::RustConn;

pub fn logout_menu() -> Box<dyn KeyEventHandler<RustConn>> {
    key_handler(|state, _x| {
        let choices = vec!["󰒲  suspend", "󰍃  logout", "󱞳  reboot", "󰤆  shutdown"];

        let config = DMenuConfig {
            ignore_case: true,
            show_line_numbers: false,
            custom_prompt: Some("Power Menu".to_string()),
            show_on_bottom: false,
            password_input: false,
            ..DMenuConfig::default()
        };

        let screen_index = state.client_set.current_screen().index();
        let dmenu = DMenu::new(&config, screen_index);

        if let Ok(MenuMatch::Line(_, choice)) = dmenu.build_menu(choices) {
            // Clean WM shutdown: penrose stops wm.run() and main returns, ending
            // the X session -- same as M-S-q. `kill -9 -1` used to SIGKILL every
            // user process (tmux/ssh included) with no cleanup.
            if choice.as_str() == "󰍃  logout" {
                let mut do_exit = exit::<RustConn>();
                return do_exit.call(state, _x);
            }

            let cmd = match choice.as_str() {
                "󰒲  suspend" => "loginctl suspend",
                "󰤆  shutdown" => "loginctl poweroff",
                "󱞳  reboot" => "loginctl reboot",
                _ => return Ok(()),
            };

            if let Err(e) = spawn(cmd) {
                tracing::error!("Failed to execute {}: {}", cmd, e);
            }
        }

        Ok(())
    })
}

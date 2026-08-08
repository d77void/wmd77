use penrose::builtin::actions::key_handler;
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
            let cmd = match choice.as_str() {
                "󰒲  suspend" => "loginctl suspend",
                "󰍃  logout" => "kill -9 -1",
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

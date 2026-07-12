use penrose::x11rb::RustConn;
use penrose::core::bindings::KeyEventHandler;
use penrose::builtin::actions::{
    exit, modify_with, send_layout_message, spawn as spawn_action,
    log_current_state,
};
use penrose::builtin::layout::messages::{ExpandMain, IncMain, ShrinkMain};
use penrose::extensions::actions::toggle_fullscreen;
use penrose::extensions::hooks::ToggleNamedScratchPad;
use penrose::map;
use std::collections::HashMap;
use crate::menus;

type KeyHandler = Box<dyn KeyEventHandler<RustConn>>;

pub fn raw_key_bindings(
    toggle_scratchpad: ToggleNamedScratchPad,
) -> HashMap<String, KeyHandler> {
    let mut raw_bindings = map! {
        map_keys: |k: &str| k.to_string();

        // Window management
        "M-j" => modify_with(|cs| cs.focus_down()),
        "M-k" => modify_with(|cs| cs.focus_up()),
        "M-S-j" => modify_with(|cs| cs.swap_down()),
        "M-S-k" => modify_with(|cs| cs.swap_up()),
        "M-q" => modify_with(|cs| cs.kill_focused()),
        "M-Tab" => modify_with(|cs| cs.toggle_tag()),
        
        // Screen management
        "M-period" => modify_with(|cs| cs.next_screen()),
        "M-comma" => modify_with(|cs| cs.previous_screen()),
        
        // Layout management
        "M-m" => modify_with(|cs| cs.next_layout()),
        "M-S-m" => modify_with(|cs| cs.previous_layout()),
        "M-Up" => send_layout_message(|| IncMain(1)),
        "M-Down" => send_layout_message(|| IncMain(-1)),
        "M-Right" => send_layout_message(|| ExpandMain),
        "M-Left" => send_layout_message(|| ShrinkMain),
        
        // Applications
        "M-d" => spawn_action("dmenu_run"),
        "M-t" => spawn_action("slock"),
        "M-Return" => spawn_action("st"),
        
        // Features
        "M-S-f" => toggle_fullscreen(),
        "M-s" => Box::new(toggle_scratchpad),
        "M-x" => menus::logout_menu(),
        
        // System
        "M-S-s" => log_current_state(),
        "M-S-q" => exit(),

        // Volume (PulseAudio)
        "XF86AudioRaiseVolume" => spawn_action("pactl set-sink-volume @DEFAULT_SINK@ +5%"),
        "XF86AudioLowerVolume" => spawn_action("pactl set-sink-volume @DEFAULT_SINK@ -5%"),
        "XF86AudioMute" => spawn_action("pactl set-sink-mute @DEFAULT_SINK@ toggle"),
    };

    // Add workspace bindings
    for tag in &["1", "2", "3", "4", "5", "6", "7", "8", "9"] {
        raw_bindings.extend([
            (
                format!("M-{tag}"),
                modify_with(move |cs| cs.focus_tag(tag)),
            ),
            (
                format!("M-S-{tag}"),
                modify_with(move |cs| cs.move_focused_to_tag(tag)),
            ),
        ]);
    }

    raw_bindings
}

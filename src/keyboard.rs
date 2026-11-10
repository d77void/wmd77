//! The `[keyboard]` section of `config.toml`: XKB layout/variant/model/options
//! (applied with `setxkbmap`) and key repeat delay/rate (applied with
//! `xset r rate`).
//!
//! Every field is optional, and nothing is run for a part that has no field
//! set: without a `[keyboard]` section the keyboard is left exactly as the
//! X server (xorg.conf, `~/.xinitrc`, ...) already set it up. Settings are
//! applied at startup, before the keybindings are grabbed, so the keymap
//! those are resolved against is already the configured one - and again on
//! every restart, since that re-runs startup from scratch.
use serde::Deserialize;
use std::process::Command;
use tracing::{info, warn};

/// X server defaults, used for whichever of `repeat_delay`/`repeat_rate` is
/// left out when the other one is set (`xset r rate` needs both).
const DEFAULT_REPEAT_DELAY: u32 = 660;
const DEFAULT_REPEAT_RATE: u32 = 25;

/// Resolved `[keyboard]` settings. `None` means "leave it as it is".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Keyboard {
    pub layout: Option<String>,
    pub variant: Option<String>,
    pub model: Option<String>,
    /// `Some(empty)` clears every XKB option the server had set.
    pub options: Option<Vec<String>>,
    /// Milliseconds before a held key starts repeating.
    pub repeat_delay: Option<u32>,
    /// Repeats per second once it does.
    pub repeat_rate: Option<u32>,
}

/// `options` can be written either as one comma-separated string
/// (`"caps:escape,grp:alt_shift_toggle"`, the way `setxkbmap` takes it) or as
/// a list (`["caps:escape", "grp:alt_shift_toggle"]`).
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawOptions {
    One(String),
    Many(Vec<String>),
}

#[derive(Debug, Default, Deserialize)]
pub struct RawKeyboard {
    layout: Option<String>,
    variant: Option<String>,
    model: Option<String>,
    options: Option<RawOptions>,
    repeat_delay: Option<u32>,
    repeat_rate: Option<u32>,
}

impl RawKeyboard {
    pub fn resolve(self) -> Keyboard {
        // An empty layout/model is almost certainly a placeholder left in
        // by hand; passing it on would only make setxkbmap fail.
        let non_empty =
            |s: Option<String>| s.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());

        let options = self.options.map(|o| {
            let items = match o {
                RawOptions::One(s) => s.split(',').map(str::to_string).collect(),
                RawOptions::Many(v) => v,
            };
            items
                .into_iter()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        });

        Keyboard {
            layout: non_empty(self.layout),
            // An empty variant is meaningful ("the layout's basic variant"),
            // so it is kept as-is.
            variant: self.variant.map(|s| s.trim().to_string()),
            model: non_empty(self.model),
            options,
            repeat_delay: self.repeat_delay.map(|d| d.clamp(1, 10_000)),
            repeat_rate: self.repeat_rate.map(|r| r.clamp(1, 255)),
        }
    }
}

impl Keyboard {
    /// The `setxkbmap` arguments for these settings, or `None` if none of
    /// the XKB fields are set.
    fn setxkbmap_args(&self) -> Option<Vec<String>> {
        let mut args = Vec::new();

        for (flag, value) in [
            ("-layout", &self.layout),
            ("-variant", &self.variant),
            ("-model", &self.model),
        ] {
            if let Some(v) = value {
                args.push(flag.to_string());
                args.push(v.clone());
            }
        }

        if let Some(options) = &self.options {
            // setxkbmap *adds* -option values to the ones already set; an
            // empty -option first clears those, so the list in config.toml
            // is the whole list.
            args.push("-option".to_string());
            args.push(String::new());
            if !options.is_empty() {
                args.push("-option".to_string());
                args.push(options.join(","));
            }
        }

        (!args.is_empty()).then_some(args)
    }

    /// The `xset` arguments for the repeat settings, or `None` if neither is
    /// set.
    fn xset_args(&self) -> Option<Vec<String>> {
        if self.repeat_delay.is_none() && self.repeat_rate.is_none() {
            return None;
        }

        Some(vec![
            "r".to_string(),
            "rate".to_string(),
            self.repeat_delay
                .unwrap_or(DEFAULT_REPEAT_DELAY)
                .to_string(),
            self.repeat_rate.unwrap_or(DEFAULT_REPEAT_RATE).to_string(),
        ])
    }

    /// Runs `setxkbmap` and `xset` for whatever is set, waiting for each to
    /// finish. Failures (a tool not installed, a layout name XKB doesn't
    /// know) are logged and otherwise ignored: a keyboard setting must never
    /// stop the window manager from starting.
    pub fn apply(&self) {
        if let Some(args) = self.setxkbmap_args() {
            run("setxkbmap", &args);
        }
        if let Some(args) = self.xset_args() {
            run("xset", &args);
        }
    }
}

fn run(program: &str, args: &[String]) {
    match Command::new(program).args(args).status() {
        Ok(status) if status.success() => info!(program, ?args, "applied keyboard settings"),
        Ok(status) => warn!(program, ?args, %status, "keyboard settings not applied"),
        Err(e) => warn!(program, %e, "failed to run, keyboard settings not applied"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kb(toml_src: &str) -> Keyboard {
        toml::from_str::<RawKeyboard>(toml_src)
            .expect("valid toml")
            .resolve()
    }

    #[test]
    fn nothing_set_runs_nothing() {
        let k = kb("");
        assert_eq!(k, Keyboard::default());
        assert!(k.setxkbmap_args().is_none());
        assert!(k.xset_args().is_none());
    }

    #[test]
    fn layout_variant_and_options() {
        let k = kb(r#"
            layout = "pt,us"
            variant = "nodeadkeys,"
            options = "grp:alt_shift_toggle, caps:escape"
        "#);
        assert_eq!(
            k.setxkbmap_args().unwrap(),
            [
                "-layout",
                "pt,us",
                "-variant",
                "nodeadkeys,",
                "-option",
                "",
                "-option",
                "grp:alt_shift_toggle,caps:escape",
            ]
        );
        assert!(k.xset_args().is_none());
    }

    #[test]
    fn options_as_a_list_and_empty_list_clears() {
        let k = kb(r#"options = ["caps:escape", "compose:ralt"]"#);
        assert_eq!(
            k.options.as_deref().unwrap(),
            ["caps:escape", "compose:ralt"]
        );

        let k = kb("options = []");
        assert_eq!(k.setxkbmap_args().unwrap(), ["-option", ""]);
    }

    #[test]
    fn empty_layout_is_ignored() {
        assert!(kb(r#"layout = "  ""#).setxkbmap_args().is_none());
    }

    #[test]
    fn repeat_fills_in_the_missing_half_and_clamps() {
        assert_eq!(
            kb("repeat_delay = 250").xset_args().unwrap(),
            ["r", "rate", "250", "25"]
        );
        assert_eq!(
            kb("repeat_rate = 40").xset_args().unwrap(),
            ["r", "rate", "660", "40"]
        );
        assert_eq!(
            kb("repeat_delay = 0\nrepeat_rate = 999")
                .xset_args()
                .unwrap(),
            ["r", "rate", "1", "255"]
        );
    }
}

//! User-editable configuration, loaded from `~/.config/wmd77/config.toml`.
//!
//! The format is the same as pwm's (theme, apps, window/float rules,
//! keybindings), so tools such as Kantele can treat the two alike; only the
//! `[bar]` section differs, since wmd77 keeps its own dwm-style status bar.
//! The themes are pwm's too, with the same `d77` default.
//!
//! Themes only change colors and the font: unlike pwm, the startup script
//! gets no theme name, so there are no per-theme wallpapers.
//!
//! The file is read at startup; the `restart` action (M-S-r) and SIGHUP
//! re-exec wmd77 so it is read again. Anything missing, malformed or absent
//! falls back to the built-in defaults, which reproduce wmd77's original
//! compiled-in setup, rather than failing to start the window manager.
use serde::Deserialize;
use std::{collections::HashMap, env, fs, path::PathBuf};
use tracing::warn;

/// A resolved color + font theme, ready to hand to the bar and borders.
/// Colors are `0xRRGGBBAA` values, matching the format `penrose::Color`
/// expects (the alpha byte is accepted for consistency
/// with the rest of the codebase, but is ignored by the actual X11/Xft
/// drawing paths).
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub black: u32,
    pub white: u32,
    pub grey: u32,
    pub accent: u32,
    pub font: String,
}

const DEFAULT_FONT: &str = "JetBrainsMono Nerd Font";
const DEFAULT_THEME_NAME: &str = "d77";

/// Resolved application settings: which programs the "spawn a terminal" /
/// "spawn a launcher" / "lock the screen" keybindings run, and where the
/// startup script lives.
#[derive(Debug, Clone)]
pub struct Apps {
    pub terminal: String,
    pub launcher: String,
    pub locker: String,
    pub startup_script: String,
}

impl Default for Apps {
    fn default() -> Self {
        Self {
            terminal: "st".to_string(),
            launcher: "dmenu_run".to_string(),
            locker: "slock".to_string(),
            startup_script: "/etc/xdg/wmd77/startup.sh".to_string(),
        }
    }
}

/// A single "windows of this class open on this tag" rule.
#[derive(Debug, Clone)]
pub struct WindowRule {
    pub class: String,
    pub workspace: String,
}

/// wmd77 ships without window rules; `[[window_rule]]` entries add them.
fn default_window_rules() -> Vec<WindowRule> {
    Vec::new()
}

/// Built-in status bar settings.
#[derive(Debug, Clone, PartialEq)]
pub struct Bar {
    /// Draw wmd77's own dwm-style bar (its text comes from the root window
    /// name, set by `/etc/xdg/wmd77/bar.sh` through `xsetroot -name`).
    pub enable: bool,
    pub height: u32,
    pub font_size: u8,
    pub bottom: bool,
    /// Pixels kept free at the top of every screen for windows to stay
    /// clear of: the built-in bar's space when it is on, or room for an
    /// external bar (polybar, lemonbar, ...) when it is off.
    pub reserve_top: u32,
}

const DEFAULT_BAR_HEIGHT: u32 = 20;
const DEFAULT_BAR_RESERVE: u32 = 18;

impl Default for Bar {
    fn default() -> Self {
        Self {
            enable: true,
            height: DEFAULT_BAR_HEIGHT,
            font_size: 12,
            bottom: false,
            reserve_top: DEFAULT_BAR_RESERVE,
        }
    }
}

/// A single "windows of this class float, centered, instead of tiling"
/// rule. Unlike [`WindowRule`], there's no built-in list of these - a
/// window only floats if `config.toml` adds a rule for its class.
#[derive(Debug, Clone)]
pub struct FloatRule {
    pub class: String,
    pub width_ratio: f64,
    pub height_ratio: f64,
}

const DEFAULT_FLOAT_RATIO: f64 = 0.6;

/// A single keybinding: `key` is the same string format
/// `parse_keybindings_with_xmodmap` already accepts (e.g. `"M-j"`), `action`
/// names one of keybindings.rs's built-in actions, and `arg` carries that
/// action's parameter when it takes one (e.g. `spawn`'s command, or
/// `inc_main`'s amount).
#[derive(Debug, Clone)]
pub struct BindSpec {
    pub key: String,
    pub action: String,
    pub arg: Option<String>,
}

/// Everything resolved from `config.toml`, ready for `main` to hand out.
#[derive(Debug, Clone)]
pub struct UserConfig {
    pub theme: Theme,
    pub apps: Apps,
    pub window_rules: Vec<WindowRule>,
    pub float_rules: Vec<FloatRule>,
    pub bar: Bar,
    /// `None` means `config.toml` had no `[[bind]]` entries at all, so
    /// keybindings.rs should use its own hardcoded defaults unchanged.
    /// `Some(specs)` is overlaid onto them key by key - see
    /// [`crate::keybindings::raw_key_bindings`].
    pub binds: Option<Vec<BindSpec>>,
}

/// The themes wmd77 ships with, the same as pwm's. `d77` is the original
/// hand-picked palette (Gruvbox Dark + a custom lavender accent); the others
/// are sourced from each project's own published palette.
fn builtin_themes() -> HashMap<String, Theme> {
    HashMap::from([
        (
            DEFAULT_THEME_NAME.to_string(),
            Theme {
                name: DEFAULT_THEME_NAME.to_string(),
                black: 0x282828ff,
                white: 0xebdbb2ff,
                grey: 0x3c3836ff,
                accent: 0xaa96daff,
                font: DEFAULT_FONT.to_string(),
            },
        ),
        (
            "gruvbox_dark".to_string(),
            Theme {
                name: "gruvbox_dark".to_string(),
                black: 0x282828ff,
                white: 0xebdbb2ff,
                grey: 0x3c3836ff,
                accent: 0xfe8019ff, // canonical Gruvbox bright orange
                font: DEFAULT_FONT.to_string(),
            },
        ),
        (
            "arc_dark".to_string(),
            Theme {
                name: "arc_dark".to_string(),
                black: 0x2f343fff,
                white: 0xd3dae3ff,
                grey: 0x353945ff,
                accent: 0x5294e2ff,
                font: DEFAULT_FONT.to_string(),
            },
        ),
        (
            "dracula".to_string(),
            Theme {
                name: "dracula".to_string(),
                black: 0x282a36ff,
                white: 0xf8f8f2ff,
                grey: 0x44475aff,
                accent: 0xbd93f9ff,
                font: DEFAULT_FONT.to_string(),
            },
        ),
        (
            "tokyo_night".to_string(),
            Theme {
                name: "tokyo_night".to_string(),
                black: 0x1a1b26ff,
                white: 0xc0caf5ff,
                grey: 0x414868ff,
                accent: 0xbb9af7ff,
                font: DEFAULT_FONT.to_string(),
            },
        ),
    ])
}

#[derive(Debug, Default, Deserialize)]
struct RawConfig {
    #[serde(default)]
    theme: RawThemeSection,
    #[serde(default)]
    apps: RawApps,
    #[serde(default, rename = "window_rule")]
    window_rules: Vec<RawWindowRule>,
    #[serde(default, rename = "float_rule")]
    float_rules: Vec<RawFloatRule>,
    #[serde(default)]
    bar: RawBar,
    #[serde(default, rename = "bind")]
    binds: Vec<RawBind>,
}

#[derive(Debug, Deserialize)]
struct RawBind {
    key: String,
    action: String,
    arg: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawWindowRule {
    class: String,
    workspace: String,
}

#[derive(Debug, Deserialize)]
struct RawFloatRule {
    class: String,
    width_ratio: Option<f64>,
    height_ratio: Option<f64>,
}

#[derive(Debug, Default, Deserialize)]
struct RawBar {
    enable: Option<bool>,
    height: Option<u32>,
    font_size: Option<u8>,
    position: Option<String>,
    reserve_top: Option<u32>,
}

impl RawBar {
    fn resolve(self) -> Bar {
        let default = Bar::default();
        let enable = self.enable.unwrap_or(default.enable);
        let bottom = match self.position.as_deref() {
            None | Some("top") => false,
            Some("bottom") => true,
            Some(other) => {
                warn!(position = other, "bar position must be \"top\" or \"bottom\", using top");
                false
            }
        };
        // With the built-in bar off (or at the bottom) nothing needs room at
        // the top unless the user asks for it, e.g. for an external bar.
        let reserve_default = if enable && !bottom { default.reserve_top } else { 0 };
        Bar {
            enable,
            height: self.height.unwrap_or(default.height).clamp(8, 200),
            font_size: self.font_size.unwrap_or(default.font_size).clamp(4, 72),
            bottom,
            reserve_top: self.reserve_top.unwrap_or(reserve_default).min(500),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct RawApps {
    terminal: Option<String>,
    launcher: Option<String>,
    locker: Option<String>,
    startup_script: Option<String>,
}

impl RawApps {
    /// Overlays whichever fields were set in `config.toml` on top of the
    /// built-in defaults.
    fn resolve(self) -> Apps {
        let default = Apps::default();
        Apps {
            terminal: self.terminal.unwrap_or(default.terminal),
            launcher: self.launcher.unwrap_or(default.launcher),
            locker: self.locker.unwrap_or(default.locker),
            startup_script: self.startup_script.unwrap_or(default.startup_script),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct RawThemeSection {
    active: Option<String>,
    // Any table under [theme.*] other than the `active` key above is
    // captured here as a user-defined (or overridden) palette.
    #[serde(flatten)]
    palettes: HashMap<String, RawTheme>,
}

#[derive(Debug, Deserialize)]
struct RawTheme {
    black: String,
    white: String,
    grey: String,
    accent: String,
    #[serde(default = "default_font")]
    font: String,
}

fn default_font() -> String {
    DEFAULT_FONT.to_string()
}

/// Parses a color string in `0xRRGGBB[AA]` or `#RRGGBB[AA]` form (the `0x`/`#`
/// prefix is optional) into the `0xRRGGBBAA` form `penrose::Color` expects.
fn parse_hex_color(raw: &str) -> Option<u32> {
    let cleaned = raw.trim().trim_start_matches("0x").trim_start_matches('#');

    match cleaned.len() {
        6 => u32::from_str_radix(cleaned, 16)
            .ok()
            .map(|rgb| (rgb << 8) | 0xff),
        8 => u32::from_str_radix(cleaned, 16).ok(),
        _ => None,
    }
}

impl RawTheme {
    /// Converts to a resolved [`Theme`], falling back to `fallback`'s fields
    /// one at a time for anything that fails to parse, so a single typo
    /// doesn't take out the whole palette.
    fn resolve(&self, name: &str, fallback: &Theme) -> Theme {
        let field = |raw: &str, field_name: &str, default: u32| match parse_hex_color(raw) {
            Some(v) => v,
            None => {
                warn!(
                    theme = name,
                    field = field_name,
                    value = raw,
                    "invalid color, using fallback"
                );
                default
            }
        };

        Theme {
            name: name.to_string(),
            black: field(&self.black, "black", fallback.black),
            white: field(&self.white, "white", fallback.white),
            grey: field(&self.grey, "grey", fallback.grey),
            accent: field(&self.accent, "accent", fallback.accent),
            font: self.font.clone(),
        }
    }
}

/// Resolves the `~/.config/wmd77/config.toml` path, honoring `$XDG_CONFIG_HOME`.
fn config_path() -> Option<PathBuf> {
    let base = match env::var_os("XDG_CONFIG_HOME") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => PathBuf::from(env::var_os("HOME")?).join(".config"),
    };

    Some(base.join("wmd77").join("config.toml"))
}

fn read_raw_config() -> RawConfig {
    let Some(path) = config_path() else {
        return RawConfig::default();
    };

    let Ok(contents) = fs::read_to_string(&path) else {
        // Not finding a config file is the common case (nobody has written
        // one yet) - nothing to warn about.
        return RawConfig::default();
    };

    parse_raw(&contents).unwrap_or_else(|e| {
        warn!(path = %path.display(), error = %e, "failed to parse config.toml, using defaults");
        RawConfig::default()
    })
}

fn parse_raw(contents: &str) -> Result<RawConfig, toml::de::Error> {
    toml::from_str(contents)
}

/// Resolves the active theme: built-in themes, overlaid with any palettes
/// defined in `config.toml`, selecting whichever one `[theme].active` names
/// (defaulting to `"d77"`, and falling back to it if the requested name
/// doesn't exist).
fn resolve_theme(raw: RawThemeSection) -> Theme {
    let mut themes = builtin_themes();
    let default_theme = themes[DEFAULT_THEME_NAME].clone();

    for (name, raw_theme) in &raw.palettes {
        let fallback = themes.get(name).unwrap_or(&default_theme).clone();
        themes.insert(name.clone(), raw_theme.resolve(name, &fallback));
    }

    let active = raw.active.as_deref().unwrap_or(DEFAULT_THEME_NAME);

    match themes.get(active) {
        Some(theme) => theme.clone(),
        None => {
            warn!(
                active,
                "unknown theme, falling back to '{DEFAULT_THEME_NAME}'"
            );
            default_theme
        }
    }
}

/// Resolves window rules: `config.toml`'s `[[window_rule]]` entries are
/// overlaid onto the built-in defaults by `class` - a rule for a class the
/// defaults already cover replaces just its `workspace`, a new class is
/// added, and any class not mentioned keeps its default rule untouched.
fn resolve_window_rules(raw: Vec<RawWindowRule>) -> Vec<WindowRule> {
    let mut rules = default_window_rules();

    for r in raw {
        match rules.iter_mut().find(|existing| existing.class == r.class) {
            Some(existing) => existing.workspace = r.workspace,
            None => rules.push(WindowRule {
                class: r.class,
                workspace: r.workspace,
            }),
        }
    }

    rules
}

/// Resolves float rules from `config.toml`'s `[[float_rule]]` entries.
/// There's no built-in list to overlay onto (unlike [`resolve_window_rules`]):
/// a class only shows up here if the user asked for it, and a rule missing
/// `width_ratio`/`height_ratio` falls back to [`DEFAULT_FLOAT_RATIO`] for
/// whichever one it left out. Values outside `0.0..=1.0` are clamped, since
/// `FloatingCentered::new` panics on them and a mistyped ratio shouldn't be
/// able to crash wmd77 on startup.
fn resolve_float_rules(raw: Vec<RawFloatRule>) -> Vec<FloatRule> {
    raw.into_iter()
        .map(|r| {
            let clamp = |ratio: Option<f64>| ratio.unwrap_or(DEFAULT_FLOAT_RATIO).clamp(0.0, 1.0);
            FloatRule {
                class: r.class,
                width_ratio: clamp(r.width_ratio),
                height_ratio: clamp(r.height_ratio),
            }
        })
        .collect()
}

/// `None` if `config.toml` has no `[[bind]]` entries (use the built-in
/// keymap unchanged); `Some` with a fully-replacing list otherwise.
fn resolve_binds(raw: Vec<RawBind>) -> Option<Vec<BindSpec>> {
    if raw.is_empty() {
        return None;
    }

    Some(
        raw.into_iter()
            .map(|b| BindSpec {
                key: b.key,
                action: b.action,
                arg: b.arg,
            })
            .collect(),
    )
}

/// Reads `config.toml` once (see [`read_raw_config`]) and resolves every
/// section it can be missing or invalid in from it.
pub fn load() -> UserConfig {
    resolve(read_raw_config())
}

fn resolve(raw: RawConfig) -> UserConfig {
    UserConfig {
        theme: resolve_theme(raw.theme),
        apps: raw.apps.resolve(),
        window_rules: resolve_window_rules(raw.window_rules),
        float_rules: resolve_float_rules(raw.float_rules),
        bar: raw.bar.resolve(),
        binds: resolve_binds(raw.binds),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(toml: &str) -> UserConfig {
        resolve(parse_raw(toml).expect("valid toml"))
    }

    #[test]
    fn empty_config_reproduces_the_compiled_in_setup() {
        let c = cfg("");
        assert_eq!(c.theme.name, "d77");
        assert_eq!(c.theme.black, 0x282828ff);
        assert_eq!(c.theme.accent, 0xaa96daff);
        assert_eq!(c.theme.grey, 0x3c3836ff);
        assert_eq!(c.theme.font, "JetBrainsMono Nerd Font");
        assert_eq!(c.apps.terminal, "st");
        assert_eq!(c.apps.launcher, "dmenu_run");
        assert_eq!(c.apps.locker, "slock");
        assert_eq!(c.apps.startup_script, "/etc/xdg/wmd77/startup.sh");
        assert!(c.window_rules.is_empty());
        assert!(c.float_rules.is_empty());
        assert_eq!(c.bar, Bar::default());
        assert!(c.binds.is_none());
    }

    #[test]
    fn builtin_theme_by_name() {
        let c = cfg("[theme]\nactive = \"dracula\"\n");
        assert_eq!(c.theme.accent, 0xbd93f9ff);
    }

    #[test]
    fn own_theme() {
        let c = cfg(r##"
            [theme]
            active = "mine"
            [theme.mine]
            black = "#000000"
            white = "0xffffff"
            grey = "#111111ff"
            accent = "#ff0000"
        "##);
        assert_eq!(c.theme.white, 0xffffffff);
        assert_eq!(c.theme.accent, 0xff0000ff);
        assert_eq!(c.theme.font, "JetBrainsMono Nerd Font");
    }

    #[test]
    fn bad_color_falls_back_field_by_field() {
        let c = cfg(r##"
            [theme]
            active = "d77"
            [theme.d77]
            black = "nope"
            white = "#ffffff"
            grey = "#111111"
            accent = "#ff0000"
        "##);
        assert_eq!(c.theme.black, 0x282828ff);
        assert_eq!(c.theme.white, 0xffffffff);
    }

    #[test]
    fn unknown_theme_falls_back_to_d77() {
        assert_eq!(cfg("[theme]\nactive = \"nope\"\n").theme.name, "d77");
    }

    #[test]
    fn bar_off_reserves_nothing_unless_asked() {
        let c = cfg("[bar]\nenable = false\n");
        assert!(!c.bar.enable);
        assert_eq!(c.bar.reserve_top, 0);

        let c = cfg("[bar]\nenable = false\nreserve_top = 30\n");
        assert_eq!(c.bar.reserve_top, 30);
    }

    #[test]
    fn bar_at_the_bottom() {
        let c = cfg("[bar]\nposition = \"bottom\"\nheight = 24\n");
        assert!(c.bar.bottom);
        assert_eq!(c.bar.height, 24);
        assert_eq!(c.bar.reserve_top, 0);
        assert!(!cfg("[bar]\nposition = \"sideways\"\n").bar.bottom);
    }

    #[test]
    fn rules_and_binds() {
        let c = cfg(r#"
            [[window_rule]]
            class = "firefox"
            workspace = "9"
            [[window_rule]]
            class = "firefox"
            workspace = "2"

            [[float_rule]]
            class = "Pavucontrol"
            width_ratio = 1.7

            [[bind]]
            key = "M-r"
            action = "spawn"
            arg = "d77run"
        "#);
        assert_eq!(c.window_rules.len(), 1);
        assert_eq!(c.window_rules[0].workspace, "2");
        assert_eq!(c.float_rules[0].width_ratio, 1.0);
        assert_eq!(c.float_rules[0].height_ratio, DEFAULT_FLOAT_RATIO);
        let binds = c.binds.expect("binds");
        assert_eq!(binds[0].key, "M-r");
        assert_eq!(binds[0].arg.as_deref(), Some("d77run"));
    }

    #[test]
    fn shipped_example_parses_to_the_defaults() {
        let example = include_str!("../usr/share/wmd77/config.toml.example");
        let c = resolve(parse_raw(example).expect("example parses"));
        assert_eq!(c.theme.name, "d77");
        assert_eq!(c.bar, Bar::default());
        assert!(c.binds.is_none());
    }

    #[test]
    fn invalid_toml_is_an_error_not_a_panic() {
        assert!(parse_raw("[theme\n").is_err());
    }
}

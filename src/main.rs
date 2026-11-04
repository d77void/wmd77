mod config;
mod hooks;
mod keybindings;
mod layouts;
mod menus;
mod mouse;
mod reload;
mod scratchpads;
mod user_config;

use anyhow::{Context, Result};
use nix::sys::signal::{SigHandler, Signal, signal};
use penrose::core::{WindowManager, bindings::parse_keybindings_with_xmodmap};
use penrose::extensions::hooks::add_named_scratchpads;
use penrose::x11rb::RustConn;
use penrose_ui::core::TextStyle;
use penrose_ui::{bar::Position, status_bar};
use std::os::unix::process::CommandExt;
use std::sync::atomic::Ordering;
use tracing::{info, warn};
use tracing_subscriber::{self, prelude::*};

use keybindings::RESTART_REQUESTED;

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("trace")
        .finish()
        .init();

    // ~/.config/wmd77/config.toml, or the built-in defaults
    let user = user_config::load();
    let theme = &user.theme;
    info!(theme = %theme.name, bar = user.bar.enable, "loaded configuration");

    // Create scratchpad
    let (nsp_terminal, toggle_terminal) = scratchpads::create_terminal_scratchpad();

    // Build configuration
    let config = config::build_config(&user);

    // Setup X11 connection
    let conn = RustConn::new().context("Failed to establish X11 connection")?;

    // Parse keybindings
    let key_bindings = parse_keybindings_with_xmodmap(keybindings::raw_key_bindings(
        toggle_terminal,
        &user.apps,
        user.binds.as_deref(),
    ))
    .context("Failed to parse keybindings")?;

    // Initialize window manager
    let wm = WindowManager::new(config, key_bindings, mouse::mouse_bindings(), conn)
        .context("Failed to initialize window manager")?;

    // The built-in status bar ([bar] enable = false leaves room for an
    // external one instead, see [bar] reserve_top)
    let wm = if user.bar.enable {
        let style = TextStyle {
            fg: theme.white.into(),
            bg: Some(theme.black.into()),
            padding: (2, 2),
        };
        let position = if user.bar.bottom {
            Position::Bottom
        } else {
            Position::Top
        };
        let bar = status_bar(
            user.bar.height,
            &theme.font,
            user.bar.font_size,
            style,
            theme.accent,
            theme.grey,
            position,
        )
        .map_err(|e| anyhow::anyhow!("Failed to create status bar: {}", e))?;
        bar.add_to(wm)
    } else {
        wm
    };

    // Add scratchpads
    let wm = add_named_scratchpads(wm, vec![nsp_terminal]);

    // Run window manager
    reload::install();
    wm.run().context("Window manager event loop failed")?;

    // The `restart` action (M-S-r) and SIGHUP stop the loop above like a
    // normal exit, but set RESTART_REQUESTED first. The X connection is gone
    // by now, so re-exec'ing into a fresh wmd77 is safe: it re-reads
    // config.toml and adopts every window that is still mapped.
    if RESTART_REQUESTED.load(Ordering::SeqCst) {
        // wm.run() sets SIGCHLD to SIG_IGN, which survives exec(); the new
        // process would then fail waiting on xmodmap at startup
        // (parse_keybindings_with_xmodmap) with ECHILD. Reset it first.
        //
        // SAFETY: nothing else handles SIGCHLD at this point.
        if let Err(e) = unsafe { signal(Signal::SIGCHLD, SigHandler::SigDfl) } {
            warn!(%e, "failed to reset SIGCHLD disposition before restart");
        }

        let exe = std::env::current_exe()?;
        let err = std::process::Command::new(exe).exec();
        // `exec` only returns if it failed to replace the process image.
        panic!("failed to re-exec wmd77 for restart: {err}");
    }

    Ok(())
}

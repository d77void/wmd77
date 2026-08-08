mod config;
mod hooks;
mod keybindings;
mod layouts;
mod menus;
mod mouse;
mod scratchpads;
mod theme;

use anyhow::{Context, Result};
use penrose::core::{WindowManager, bindings::parse_keybindings_with_xmodmap};
use penrose::extensions::hooks::add_named_scratchpads;
use penrose::x11rb::RustConn;
use penrose_ui::{bar::Position, status_bar};
use tracing_subscriber::{self, prelude::*};

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("trace")
        .finish()
        .init();

    // Create scratchpad
    let (nsp_terminal, toggle_terminal) = scratchpads::create_terminal_scratchpad();

    // Build configuration
    let config = config::build_config();

    // Setup X11 connection
    let conn = RustConn::new().context("Failed to establish X11 connection")?;

    // Parse keybindings
    let key_bindings =
        parse_keybindings_with_xmodmap(keybindings::raw_key_bindings(toggle_terminal))
            .context("Failed to parse keybindings")?;

    // Create status bar
    let bar = status_bar(
        theme::BAR_HEIGHT_PX,
        theme::FONT,
        12,
        theme::bar_style(),
        theme::BLUE,
        theme::GREY,
        Position::Top,
    )
    .map_err(|e| anyhow::anyhow!("Failed to create status bar: {}", e))?;

    // Initialize window manager
    let wm = WindowManager::new(config, key_bindings, mouse::mouse_bindings(), conn)
        .context("Failed to initialize window manager")?;

    // Add status bar to window manager
    let wm = bar.add_to(wm);

    // Add scratchpads
    let wm = add_named_scratchpads(wm, vec![nsp_terminal]);

    // Run window manager
    wm.run().context("Window manager event loop failed")?;

    Ok(())
}

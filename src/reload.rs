//! Reload `config.toml` on `SIGHUP`, so external tools (e.g. a settings GUI)
//! can apply changes without the user pressing the `restart` keybinding.
//!
//! Penrose's event loop blocks on the X connection, so a signal alone can't
//! reach it. Three small pieces cooperate instead:
//!
//! 1. the `SIGHUP` handler only flips an atomic flag (the only thing that is
//!    async-signal-safe to do there);
//! 2. a watcher thread notices the flag and touches a property on the root
//!    window through its own X connection, which wakes the event loop with a
//!    `PropertyNotify` (penrose selects `PROPERTY_CHANGE` on the root);
//! 3. an event hook sees the flag on that event and runs the very same
//!    `restart` the `M-S-r` keybinding does, so `main` re-execs wmd77 in place.
//!
//! The handler is a plain function, not a blocked signal mask, on purpose: a
//! blocked `SIGHUP` would be inherited by every program wmd77 spawns.

use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use nix::libc::c_int;
use nix::sys::signal::{SaFlags, SigAction, SigHandler, SigSet, Signal, sigaction};
use penrose::builtin::actions::exit;
use penrose::core::State;
use penrose::x::{Prop, XConn, XEvent};
use penrose::x11rb::RustConn;
use tracing::{info, warn};

use crate::keybindings::RESTART_REQUESTED;

/// Root window property touched to wake the event loop. Its value is
/// irrelevant; only the resulting `PropertyNotify` matters.
const WAKE_PROP: &str = "_WMD77_RELOAD";

/// How often the watcher thread checks for a pending reload.
const POLL_INTERVAL: Duration = Duration::from_millis(200);

static SIGHUP_RECEIVED: AtomicBool = AtomicBool::new(false);

extern "C" fn on_sighup(_: c_int) {
    SIGHUP_RECEIVED.store(true, Ordering::SeqCst);
}

/// Installs the `SIGHUP` handler and starts the watcher thread. Must run
/// before `WindowManager::run`. A failure here only costs the reload
/// feature, never startup: it is logged and wmd77 carries on.
pub fn install() {
    // SA_RESTART: the X connection's blocking reads resume transparently
    // instead of failing with EINTR in penrose's event loop.
    let action = SigAction::new(
        SigHandler::Handler(on_sighup),
        SaFlags::SA_RESTART,
        SigSet::empty(),
    );
    // SAFETY: the handler only performs an atomic store.
    if let Err(e) = unsafe { sigaction(Signal::SIGHUP, &action) } {
        warn!(%e, "failed to install SIGHUP handler; reload-on-SIGHUP disabled");
        return;
    }

    let spawned = thread::Builder::new()
        .name("wmd77-reload".into())
        .spawn(watch);
    if let Err(e) = spawned {
        warn!(%e, "failed to start reload watcher; reload-on-SIGHUP disabled");
    }
}

fn watch() {
    loop {
        thread::sleep(POLL_INTERVAL);
        if !SIGHUP_RECEIVED.load(Ordering::SeqCst) {
            continue;
        }
        info!("SIGHUP received, reloading config.toml");
        if let Err(e) = wake_event_loop() {
            // Without the wake-up the flag is still honoured on the next X
            // event (pointer motion, focus change, ...), just not instantly.
            warn!(%e, "failed to wake the event loop for reload");
        }
        // wmd77 is about to re-exec; nothing left to watch for.
        return;
    }
}

fn wake_event_loop() -> penrose::Result<()> {
    let conn = RustConn::new()?;
    conn.set_prop(conn.root(), WAKE_PROP, Prop::Cardinal(vec![1]))?;
    conn.flush();
    // Wait for a reply before this connection is dropped: with only the
    // flush, closing the connection right away could lose the change, and
    // then the reload waited for whatever X event came next.
    conn.get_prop(conn.root(), WAKE_PROP)?;
    Ok(())
}

/// Event hook that turns a pending `SIGHUP` into a restart. Runs ahead of
/// penrose's own handling of every event.
pub fn event_hook(_: &XEvent, state: &mut State<RustConn>, x: &RustConn) -> penrose::Result<bool> {
    if !SIGHUP_RECEIVED.swap(false, Ordering::SeqCst) {
        return Ok(true);
    }
    RESTART_REQUESTED.store(true, Ordering::SeqCst);
    exit::<RustConn>().call(state, x)?;
    Ok(false)
}

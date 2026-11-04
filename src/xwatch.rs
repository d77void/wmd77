//! Exit when the X server goes away.
//!
//! When reading the next X event fails, penrose's event loop logs the error
//! and tries again, forever: a window manager whose X server died would
//! spin at full CPU and fill its log with "Unhandled error pulling next x
//! event". This thread holds its own connection, selects no events and just
//! waits on it; once the server is gone that read fails and wmd77 exits.

use std::thread;

use tracing::{error, warn};
use x11rb::connection::Connection;

/// Starts the watcher. A failure here only costs the watchdog, never startup.
pub fn install() {
    let conn = match x11rb::connect(None) {
        Ok((conn, _)) => conn,
        Err(e) => {
            warn!(%e, "failed to open the X watchdog connection");
            return;
        }
    };

    let spawned = thread::Builder::new()
        .name("wmd77-xwatch".into())
        .spawn(move || loop {
            // No events are selected on this connection; anything that does
            // arrive (an X error, say) is ignored. Only a broken connection
            // ends the loop.
            if let Err(e) = conn.wait_for_event() {
                error!(%e, "lost the connection to the X server, exiting");
                std::process::exit(1);
            }
        });
    if let Err(e) = spawned {
        warn!(%e, "failed to start the X watchdog");
    }
}

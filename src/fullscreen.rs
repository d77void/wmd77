//! Fullscreen that follows EWMH.
//!
//! penrose 0.4.0's `set_fullscreen_state` reads and writes `_NET_WM_STATE` as
//! a CARDINAL list, but EWMH defines it as a list of ATOMs. Clients that read
//! their own state back (mpv, browsers) then never find
//! `_NET_WM_STATE_FULLSCREEN`: they believe they are still windowed, keep
//! asking to *enter* fullscreen, and their own key can't leave it.
//!
//! This module replaces penrose's handling of both the client requests
//! (`event_hook`, composed ahead of the ewmh hooks) and the `M-S-f` binding
//! (`toggle_fullscreen`), keeping `_NET_WM_STATE` an ATOM list.
use penrose::{
    Error, Result, Xid,
    builtin::actions::key_handler,
    core::{State, bindings::KeyEventHandler},
    x::{ClientConfig, XConn, XConnExt, XEvent, atom::Atom, property::Prop},
};

const FULLSCREEN: &str = "_NET_WM_STATE_FULLSCREEN";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Remove,
    Add,
    Toggle,
}

/// The client's `_NET_WM_STATE` atoms. A CARDINAL list (left by penrose's own
/// handler) is mapped back to atom names so it gets rewritten correctly.
fn wm_state<X: XConn>(id: Xid, x: &X) -> Vec<String> {
    match x.get_prop(id, Atom::NetWmState.as_ref()) {
        Ok(Some(Prop::Atom(atoms))) => atoms,
        Ok(Some(Prop::Cardinal(values))) => values
            .into_iter()
            .filter_map(|v| x.atom_name(Xid::from(v)).ok())
            .collect(),
        _ => vec![],
    }
}

fn set_fullscreen<X: XConn>(id: Xid, action: Action, state: &mut State<X>, x: &X) -> Result<()> {
    let mut atoms = wm_state(id, x);
    let current = atoms.iter().any(|a| a == FULLSCREEN);
    let wanted = match action {
        Action::Add => true,
        Action::Remove => false,
        Action::Toggle => !current,
    };

    if wanted && !current {
        let r = state
            .client_set
            .screen_for_client(&id)
            .ok_or(Error::UnknownClient(id))?
            .geometry();
        state.client_set.float(id, r)?;
        atoms.push(FULLSCREEN.to_owned());
        x.set_client_config(id, &[ClientConfig::BorderPx(0)])?;
    } else if !wanted && current {
        state.client_set.sink(&id);
        atoms.retain(|a| a != FULLSCREEN);
        let border = state.config.border_width;
        x.set_client_config(id, &[ClientConfig::BorderPx(border)])?;
    }

    // Always rewrite: the client must see the state it asked for, as ATOMs.
    x.set_prop(id, Atom::NetWmState.as_ref(), Prop::Atom(atoms))?;
    x.refresh(state)
}

/// Handles `_NET_WM_STATE` fullscreen requests from clients and stops them
/// from reaching penrose's ewmh hook. Everything else passes through.
pub fn event_hook<X: XConn>(event: &XEvent, state: &mut State<X>, x: &X) -> Result<bool> {
    let message = match event {
        XEvent::ClientMessage(m) if m.dtype == Atom::NetWmState.as_ref() => m,
        _ => return Ok(true),
    };

    // data32: [action, first property, second property, source, 0]
    let data = message.data.as_u32();
    let fullscreen = *x.intern_atom(FULLSCREEN)?;
    if data.len() < 3
        || !(data[1] == fullscreen || data[2] == fullscreen)
        || !state.client_set.contains(&message.id)
    {
        return Ok(true);
    }

    let action = match data[0] {
        0 => Action::Remove,
        1 => Action::Add,
        2 => Action::Toggle,
        _ => return Ok(false),
    };
    set_fullscreen(message.id, action, state, x)?;
    Ok(false)
}

/// Toggle fullscreen on the focused client (replaces penrose's
/// `extensions::actions::toggle_fullscreen`).
pub fn toggle_fullscreen<X: XConn>() -> Box<dyn KeyEventHandler<X>> {
    key_handler(|state, x: &X| {
        let id = match state.client_set.current_client() {
            Some(&id) => id,
            None => return Ok(()),
        };
        set_fullscreen(id, Action::Toggle, state, x)
    })
}

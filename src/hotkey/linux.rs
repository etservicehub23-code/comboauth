//! Linux hotkey registration.
//!
//! X11: delegates to `global-hotkey` (XCB backend).
//! Wayland: attempts the XDG GlobalShortcuts portal via `ashpd`. If the
//! compositor does not support the portal (e.g. compositors older than
//! the portal spec), logs a warning and returns — the user must trigger
//! paste via the tray icon or IPC instead.

use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

use crate::paste::is_wayland_session;

/// Spawn a background listener for the Ctrl+K hotkey and call `on_trigger`
/// on each press. On Wayland, falls back to the ashpd GlobalShortcuts portal;
/// if the portal is unavailable a warning is logged and no hotkey is registered.
pub fn spawn_listener(on_trigger: impl Fn() + Send + 'static) -> Result<(), Box<dyn std::error::Error>> {
    if is_wayland_session() {
        spawn_wayland_listener(on_trigger);
        Ok(())
    } else {
        spawn_x11_listener(on_trigger)
    }
}

fn spawn_x11_listener(on_trigger: impl Fn() + Send + 'static) -> Result<(), Box<dyn std::error::Error>> {
    let manager = GlobalHotKeyManager::new()?;
    let hotkey = HotKey::new(Some(Modifiers::CONTROL), Code::KeyK);
    manager.register(hotkey)?;
    // Keep alive for the life of the process — drop unregisters.
    std::mem::forget(manager);

    std::thread::spawn(move || {
        let receiver = GlobalHotKeyEvent::receiver();
        loop {
            if let Ok(event) = receiver.recv() {
                if event.id() == hotkey.id() && event.state == HotKeyState::Pressed {
                    on_trigger();
                }
            }
        }
    });
    Ok(())
}

/// Spawns a background thread that runs a tokio runtime to drive the async
/// ashpd portal. Portal unavailability is treated as a soft failure: the
/// thread logs the issue and exits cleanly so the daemon stays up.
pub fn spawn_wayland_listener(on_trigger: impl Fn() + Send + 'static) {
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                eprintln!("comboauth: failed to start runtime for Wayland hotkey: {e}");
                return;
            }
        };
        if let Err(e) = rt.block_on(wayland_portal_listener(on_trigger)) {
            eprintln!(
                "comboauth: Wayland GlobalShortcuts portal unavailable — Ctrl+K not registered: {e}"
            );
            eprintln!("comboauth: Use the tray icon or IPC to trigger paste on this Wayland session.");
        }
    });
}

async fn wayland_portal_listener(
    on_trigger: impl Fn() + Send + 'static,
) -> Result<(), Box<dyn std::error::Error>> {
    use ashpd::desktop::CreateSessionOptions;
    use ashpd::desktop::global_shortcuts::{BindShortcutsOptions, GlobalShortcuts, NewShortcut};
    use futures_util::StreamExt as _;

    let proxy = GlobalShortcuts::new().await?;
    let session = proxy.create_session(CreateSessionOptions::default()).await?;

    let shortcuts = [
        // preferred_trigger uses XDG shortcut spec format (<Control>k).
        // Compositors that support it will pre-bind Ctrl+K; others let the user choose.
        NewShortcut::new("ctrl-k", "Open ComboAuth combo picker")
            .preferred_trigger("<Control>k"),
    ];
    // bind_shortcuts awaits the portal response (compositor may show a dialog).
    let req = proxy
        .bind_shortcuts(&session, &shortcuts, None, BindShortcutsOptions::default())
        .await?;
    let bound = req.response()?;
    if bound.shortcuts().is_empty() {
        return Err("Wayland portal bound 0 shortcuts — user may have declined or compositor rejected the binding".into());
    }
    eprintln!(
        "comboauth: Wayland GlobalShortcuts portal active — {} shortcut(s) bound",
        bound.shortcuts().len()
    );

    let mut stream = proxy.receive_activated().await?;
    // _session must stay alive to keep the portal session open.
    let _session = session;
    while let Some(activation) = stream.next().await {
        if activation.shortcut_id() == "ctrl-k" {
            on_trigger();
        }
    }
    Ok(())
}

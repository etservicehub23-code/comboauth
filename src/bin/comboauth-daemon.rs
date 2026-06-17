//! comboauth-daemon — background process owning the global Ctrl+K hotkey
//! and the IPC server used by comboauth-tray.
//!
//! Scope note (Phase 9-B): this wires up real, independently-testable
//! infrastructure — AX permission request, global hotkey registration,
//! focused-field classification, clipboard paste, and the IPC protocol
//! against the real secret store. The floating "type your combo, see it
//! matched" picker overlay is NOT implemented yet: today, Ctrl+K logs the
//! detected field kind, and secret delivery happens via the `PasteSelected`
//! IPC request (callable from comboauth-tray once 9-C wires a menu for it).
//! Capturing the full combo sequence globally requires a lower-level
//! keyboard tap and is deferred to a follow-up phase so it can be designed
//! and tested against the real picker UX rather than guessed at here.

use std::sync::Arc;

use comboauth::ipc::{DaemonRequest, DaemonResponse, socket_path};
use comboauth::service::ServiceId;
use comboauth::vault::SecretStore;
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener;
use tokio::sync::Mutex;

#[allow(unreachable_code)]
fn build_secret_store() -> Box<dyn SecretStore + Send + Sync> {
    #[cfg(all(target_os = "macos", feature = "macos-keychain"))]
    {
        return Box::new(comboauth::vault::macos_keychain::MacosKeychainStore);
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(store) = comboauth::vault::linux_oo7::OsSecretStore::new() {
            return Box::new(store);
        }
    }
    eprintln!("comboauth-daemon: OS keychain unavailable — using in-memory mock store");
    Box::new(comboauth::vault::mock::MockSecretStore::default())
}

/// Registers the global Ctrl+K hotkey and spawns a blocking thread that
/// forwards trigger events into the async world via `on_trigger`.
fn spawn_hotkey_listener(on_trigger: impl Fn() + Send + 'static) -> Result<(), Box<dyn std::error::Error>> {
    let manager = GlobalHotKeyManager::new()?;
    let hotkey = HotKey::new(Some(Modifiers::CONTROL), Code::KeyK);
    manager.register(hotkey)?;
    // Keep the manager alive for the life of the process — dropping it
    // unregisters the hotkey.
    std::mem::forget(manager);

    std::thread::spawn(move || {
        let receiver = GlobalHotKeyEvent::receiver();
        loop {
            if let Ok(event) = receiver.recv() {
                if event.id() == hotkey.id() {
                    on_trigger();
                }
            }
        }
    });
    Ok(())
}

fn on_hotkey_triggered() {
    let field_kind = comboauth::focus::focused_field_kind();
    eprintln!("comboauth-daemon: Ctrl+K triggered, focused field = {field_kind:?}");
    // TODO(follow-up phase): open the picker overlay here instead of just logging.
}

async fn handle_connection(
    mut stream: tokio::net::UnixStream,
    secret_store: Arc<Mutex<Box<dyn SecretStore + Send + Sync>>>,
) {
    let mut buf = Vec::new();
    if stream.read_to_end(&mut buf).await.is_err() {
        return;
    }
    let request: DaemonRequest = match serde_json::from_slice(&buf) {
        Ok(req) => req,
        Err(e) => {
            let response = DaemonResponse::Error { message: format!("bad request: {e}") };
            let _ = stream.write_all(&serde_json::to_vec(&response).unwrap_or_default()).await;
            return;
        }
    };

    let response = match request {
        DaemonRequest::Status => DaemonResponse::Status { running: true, version: env!("CARGO_PKG_VERSION").to_string() },
        DaemonRequest::Stop => {
            let response = DaemonResponse::Ok;
            let bytes = serde_json::to_vec(&response).unwrap_or_default();
            let _ = stream.write_all(&bytes).await;
            std::process::exit(0);
        }
        DaemonRequest::ShowTui => {
            match std::process::Command::new("comboauth").spawn() {
                Ok(_) => DaemonResponse::Ok,
                Err(e) => DaemonResponse::Error { message: format!("failed to launch TUI: {e}") },
            }
        }
        DaemonRequest::PasteSelected { entry_id } => {
            let store = secret_store.lock().await;
            match store.get_secret(&ServiceId(entry_id)) {
                Ok(secret) => {
                    let secret_str = String::from_utf8_lossy(secret.expose_bytes()).into_owned();
                    match comboauth::paste::paste_and_clear(&secret_str, 200) {
                        Ok(()) => DaemonResponse::Ok,
                        Err(e) => DaemonResponse::Error { message: format!("paste failed: {e}") },
                    }
                }
                Err(e) => DaemonResponse::Error { message: format!("secret lookup failed: {e}") },
            }
        }
    };

    let _ = stream.write_all(&serde_json::to_vec(&response).unwrap_or_default()).await;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "macos")]
    {
        if !comboauth::focus::macos_ax::ensure_trusted_with_prompt() {
            eprintln!(
                "comboauth-daemon: Accessibility permission not granted yet — \
                 macOS should have shown a prompt. Grant it in System Settings \
                 > Privacy & Security > Accessibility, then restart the daemon."
            );
        }
    }

    spawn_hotkey_listener(on_hotkey_triggered)?;

    let sock = socket_path();
    if let Some(parent) = sock.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let _ = std::fs::remove_file(&sock);

    let listener = UnixListener::bind(&sock)?;
    eprintln!("comboauth-daemon: listening on {}", sock.display());

    let secret_store = Arc::new(Mutex::new(build_secret_store()));

    loop {
        let (stream, _) = listener.accept().await?;
        let secret_store = secret_store.clone();
        tokio::spawn(handle_connection(stream, secret_store));
    }
}

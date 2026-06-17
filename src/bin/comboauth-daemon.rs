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
    // TEMP for debugging (#499 thread): Ctrl+K alone is a built-in Cocoa
    // text-editing binding ("delete to end of line") on macOS, which may
    // consume the keystroke before it reaches the global hotkey layer.
    // Using Ctrl+Alt+K to rule that out; revert to Ctrl+K once confirmed.
    let hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyK);
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
    eprintln!("comboauth-daemon: hotkey triggered, focused field = {field_kind:?}");
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

async fn run_ipc_server() -> Result<(), Box<dyn std::error::Error>> {
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

/// On macOS, `global-hotkey`'s Carbon backend (`RegisterEventHotKey`) only
/// delivers events while the *main thread's* CFRunLoop is being pumped.
/// `#[tokio::main]` alone never pumps it, so the hotkey would register
/// successfully but silently never fire. We run the async IPC server on a
/// background thread and dedicate the main thread to the run loop instead.
#[allow(unreachable_code)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    std::thread::spawn(|| {
        let runtime = tokio::runtime::Runtime::new().expect("failed to start tokio runtime");
        if let Err(e) = runtime.block_on(run_ipc_server()) {
            eprintln!("comboauth-daemon: IPC server stopped: {e}");
        }
    });

    #[cfg(target_os = "macos")]
    {
        // A bare CFRunLoop is not enough: RegisterEventHotKey delivers
        // events to the *application* event target, which HIToolbox only
        // dispatches once a real NSApplication run loop is driving the
        // process (this is what was still missing after the CFRunLoop fix
        // — Carbon hotkeys need the Cocoa app context, not just any run
        // loop on the main thread). NSApplicationActivationPolicy::Accessory
        // keeps it headless: no Dock icon, no menu bar.
        use objc2::MainThreadMarker;
        use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};

        let mtm = MainThreadMarker::new().expect("daemon main() must run on the main thread");
        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
        app.run();
    }

    #[cfg(not(target_os = "macos"))]
    {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(3600));
        }
    }

    Ok(())
}

//! comboauth-daemon — background process owning the global Ctrl+K hotkey
//! and the IPC server used by comboauth-tray.
//!
//! On Ctrl+K, opens a floating combo picker (macOS: see
//! `comboauth::picker::macos`) that captures a combo sequence without
//! leaking keystrokes into whatever app was focused, matches it, and
//! pastes the resulting secret. Other platforms fall back to logging the
//! focused field kind only, until their own picker lands.

use std::sync::Arc;

use comboauth::ipc::{DaemonRequest, DaemonResponse, socket_path};
use comboauth::persistence::PersistenceStore;
use comboauth::service::ServiceId;
use comboauth::vault::SecretStore;
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener;
use tokio::sync::Mutex;

type SharedSecretStore = Arc<Mutex<Box<dyn SecretStore + Send + Sync>>>;

/// Resolves the `comboauth` TUI binary next to this daemon binary, so
/// ShowTui works regardless of whether target/release is on PATH.
fn comboauth_path() -> Result<std::path::PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = exe.parent().ok_or("daemon executable has no parent directory")?;
    Ok(dir.join("comboauth"))
}

/// Launches the TUI in a fresh, real terminal window.
///
/// A plain `Command::new(path).spawn()` would inherit the daemon's own
/// stdio (or none at all, depending on how the daemon itself was
/// launched) — ratatui needs an actual TTY to enter raw mode and draw
/// the alternate screen into, so the process must own its own terminal.
#[cfg(target_os = "macos")]
fn launch_tui_in_terminal(path: std::path::PathBuf) -> Result<(), String> {
    let raw = path.display().to_string();
    let mut escaped = String::with_capacity(raw.len());
    for ch in raw.chars() {
        if ch == '\\' || ch == '"' {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    let script = format!("tell application \"Terminal\" to do script \"{escaped}\"");
    std::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg(not(target_os = "macos"))]
fn launch_tui_in_terminal(path: std::path::PathBuf) -> Result<(), String> {
    std::process::Command::new(path).spawn().map(|_| ()).map_err(|e| e.to_string())
}

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

#[allow(unreachable_code, dead_code)]
fn build_persistence_store() -> Box<dyn PersistenceStore + Send + Sync> {
    #[cfg(all(target_os = "macos", feature = "macos-keychain"))]
    {
        return Box::new(comboauth::vault::macos_keychain::MacosPersistenceStore::new());
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(store) = comboauth::vault::linux_oo7::OsPersistenceStore::new() {
            return Box::new(store);
        }
    }
    Box::new(comboauth::persistence::MockPersistenceStore::default())
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
                // global-hotkey emits both a Pressed and a Released event per
                // keypress with the same id. Only react to Pressed, otherwise
                // the queued Released event fires on_trigger() a second time
                // as soon as the first (blocking) picker session returns.
                if event.id() == hotkey.id() && event.state == HotKeyState::Pressed {
                    on_trigger();
                }
            }
        }
    });
    Ok(())
}

fn on_hotkey_triggered(secret_store: SharedSecretStore) {
    let field_kind = comboauth::focus::focused_field_kind();
    eprintln!("comboauth-daemon: Ctrl+K triggered, focused field = {field_kind:?}");

    #[cfg(target_os = "macos")]
    {
        let persistence = build_persistence_store();
        let profiles = persistence.load_profiles().unwrap_or_default();
        let registry = persistence.load_registry().unwrap_or_default();
        let store_guard = secret_store.blocking_lock();
        comboauth::picker::macos::show_picker_and_capture(profiles, registry, &**store_guard);
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = secret_store;
        eprintln!("comboauth-daemon: picker not yet implemented on this platform (Phase 9-D)");
    }
}

async fn handle_connection(mut stream: tokio::net::UnixStream, secret_store: SharedSecretStore) {
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
            match comboauth_path().and_then(launch_tui_in_terminal) {
                Ok(()) => DaemonResponse::Ok,
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

async fn run_ipc_server(secret_store: SharedSecretStore) -> Result<(), Box<dyn std::error::Error>> {
    let sock = socket_path();
    if let Some(parent) = sock.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let _ = std::fs::remove_file(&sock);

    let listener = UnixListener::bind(&sock)?;
    eprintln!("comboauth-daemon: listening on {}", sock.display());

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

    let secret_store: SharedSecretStore = Arc::new(Mutex::new(build_secret_store()));

    spawn_hotkey_listener({
        let secret_store = secret_store.clone();
        move || on_hotkey_triggered(secret_store.clone())
    })?;

    std::thread::spawn({
        let secret_store = secret_store.clone();
        move || {
            let runtime = tokio::runtime::Runtime::new().expect("failed to start tokio runtime");
            if let Err(e) = runtime.block_on(run_ipc_server(secret_store)) {
                eprintln!("comboauth-daemon: IPC server stopped: {e}");
            }
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

# ComboAuth

ComboAuth is an experimental Rust TUI for exploring arcade-style combo input as a password workflow primitive.

A user chooses a service, enters a memorable fighting-game-like combo, and ComboAuth retrieves the stored credential from the OS keychain and delivers it via clipboard (auto-cleared after 10 seconds) or stdout. Combo sequences and timing are matched in-memory; nothing sensitive is written to disk.

## MVP Scope

- Rust 2024 application.
- Ratatui and Crossterm terminal interface.
- Central app state and event loop.
- Basic home screen with menu navigation.
- Minimal combo parser with unit tests.
- OS keychain integration (GNOME Keyring on Linux, Keychain on macOS).

## Run

```bash
cargo run
```

Press `q` or `Esc` to quit.

### Global hotkey daemon + menu bar tray (macOS)

`comboauth-daemon` (global Ctrl+K hotkey, floating picker, auto-paste) and
`comboauth-tray` (menu bar icon, start/stop daemon, launch TUI) are separate
binaries. After `cargo build --release --features macos-keychain`, start
both together with:

```bash
./scripts/launch.sh
```

Safe to re-run — it skips starting either binary if it's already running.
Logs go to `/tmp/comboauth-daemon.log` and `/tmp/comboauth-tray.log` (override
with `COMBOAUTH_LOG_DIR`); binary location defaults to `target/release`
(override with `COMBOAUTH_BIN_DIR`).

### Global hotkey daemon (Linux X11)

On Linux, `comboauth-daemon` registers Ctrl+K via `global-hotkey` (X11) and
synthesizes paste via `enigo`. AT-SPI field-kind detection (via D-Bus) is used to classify the focused element before pasting; the daemon
degrades to `FieldKind::Unknown` if AT-SPI is unavailable.

Socket path: `$XDG_RUNTIME_DIR/comboauth/daemon.sock`.

**Tray runtime dependency:** `comboauth-tray` uses `tray-icon`, which on
Linux requires `libayatana-appindicator3` (preferred) or the legacy
`libappindicator3` (GTK 3) at runtime. Install the appropriate package for
your distribution before running the tray binary:

| Distribution | Package |
|---|---|
| Debian / Ubuntu | `libayatana-appindicator3-1` |
| Fedora / RHEL | `libayatana-appindicator-gtk3` |
| Arch Linux | `libappindicator` or AUR `libayatana-appindicator` |
| openSUSE | `libayatana-appindicator3-1` |

If neither library is present the tray binary will panic at startup with:
`Failed to load ayatana-appindicator3 or appindicator3 dynamic library` (with
both `libayatana-appindicator3.so.1` and `libappindicator3.so.1` failure
details). The daemon binary itself runs without the tray and does not require GTK.

## Test

```bash
cargo test
```

## Security Model

ComboAuth is not protection against malware, keyloggers, or a compromised machine.
It is a muscle-memory UX layer over the OS keychain — designed for convenience and shoulder-surfing resistance, not cryptographic security.

Secrets are stored in the native OS keychain (GNOME Keyring on Linux, Keychain on macOS).
Combo sequences are never written to disk. The audit log records activation events by service name only — no secret bytes, no combo input.
Clipboard content is automatically cleared after 10 seconds.

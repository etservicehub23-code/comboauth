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

### Wayland limitations

ComboAuth detects a Wayland session via `WAYLAND_DISPLAY` or
`XDG_SESSION_TYPE=wayland`. Several capabilities are unavailable or degraded
on Wayland due to compositor sandbox restrictions:

| Feature | X11 | Wayland |
|---|---|---|
| Global Ctrl+K hotkey registration | `global-hotkey` (XGrabKey) | XDG GlobalShortcuts portal (`ashpd`) — user must approve in compositor settings; degrades gracefully if portal is absent or denied |
| Auto-paste (Ctrl+V synthesis) | `enigo` (XTest) | **Not available** — `enigo` requires an X server for keystroke synthesis |
| Clipboard-only paste path | — | Secret copied to clipboard; desktop notification sent via `notify-send` ("Secret copied — paste manually"); clipboard cleared after 8 s |
| AT-SPI field-kind detection | Best effort; returns `Unknown` on D-Bus errors, timeouts, or unrecognized roles | Same best-effort path — no additional Wayland restriction |

**Current Linux status:** The Linux floating picker (Phase 9-D) is not yet
implemented. When Ctrl+K fires on Linux, the daemon logs
`picker not yet implemented on this platform` and takes no paste action.
The hotkey registration and paste infrastructure below is in place for when
the picker is wired up.

**Hotkey registration:** On Wayland the daemon attempts to register Ctrl+K via
the XDG GlobalShortcuts portal (`org.freedesktop.portal.GlobalShortcuts`),
supported on KDE Plasma 5.27+, GNOME 46+ (with the portal extension), and
others. If the portal is unavailable or the permission is denied, Ctrl+K is
not registered and the daemon logs a warning. The daemon does **not** crash;
it continues running and serving IPC requests normally.

**Paste path when called on Wayland:** `paste_and_clear()` detects a Wayland
session and falls back to clipboard-only: it copies the secret, fires a
desktop notification, and schedules clipboard clear after 8 seconds. You then
paste manually with Ctrl+V. Synthetic Ctrl+V via `enigo` is not available on
Wayland — it requires an X server.

**Clipboard clear timing:** The Wayland clipboard fallback clears after 8 s
(long enough to paste manually). The X11 auto-paste path clears much sooner
(~200 ms, just after synthetic Ctrl+V). The TUI clipboard path uses 10 s.

**`notify-send` dependency:** Desktop notifications on the Wayland fallback
path require `notify-send` (typically from `libnotify-bin` / `libnotify`).
If it is absent the paste still works — the notification is silently skipped.

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

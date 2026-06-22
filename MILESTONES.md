# ComboAuth Milestones

## 1. Project Scaffold

- Create the Rust application structure.
- Add Ratatui and Crossterm.
- Add a central `App` state.
- Render a first home screen.
- Verify with `cargo check` and `cargo test`.

## 2. TUI Navigation

- Add stable screens for Services, Combos, and Settings.
- Add keyboard navigation between screens.
- Keep all data mocked.
- Avoid persistence until the UI model is clear.

## 3. Combo Parser

- Expand the parser to support diagonals and named buttons.
- Add timing-window data structures.
- Add unit tests for valid, invalid, partial, and ambiguous combos.
- Keep the parser independent from the terminal UI.

## 4. Mock Vault

- Add a mock service registry.
- Map service names to fake secrets.
- Display safe placeholders only.
- Prove the app flow without handling real credentials.

## 5. Security Design Review

- Decide whether ComboAuth should be a standalone tool, a KeePass plugin, or an OS-keychain front end.
- Define a threat model.
- Decide how secrets are encrypted, unlocked, and cleared from memory.
- Do not implement real password storage before this review.

## 6. Controlled Secret Prototype

- Add local encrypted storage only after the threat model is accepted.
- Prefer OS keychain or a proven vault backend.
- Avoid global keyboard hooks until platform-specific risks are understood.
- Add integration tests around lock, unlock, and timeout behavior.

## 7. Autofill and Platform Integration

- Evaluate platform APIs separately for Linux, macOS, and Windows.
- Prefer explicit user-triggered autofill over passive global monitoring.
- Add clear audit logs without recording secrets.
- Gate every integration behind tests and configuration.

## 8. Distribution

- Package a developer-focused CLI/TUI release first.
- Publish documentation and security limitations clearly.
- Consider Steam, Itch, or plugin distribution only after the core workflow is trustworthy.

## 9. Global Hotkey Daemon + System Tray

Split the project into three binaries and implement system-wide Ctrl+K autofill and a menu bar / tray launcher.

### Phase 9-A: Architecture Split

- Add `src/bin/comboauth-daemon.rs` — background process owning hotkey registration, field detection, paste, and IPC server.
- Add `src/bin/comboauth-tray.rs` — menu bar (macOS) / system tray (Linux) process that talks to the daemon.
- Add `src/ipc.rs` — Unix socket protocol (`DaemonRequest` / `DaemonResponse` enums, JSON over tokio async stream).
- Add platform modules: `src/hotkey/`, `src/paste/`, `src/focus/` each with `mod.rs`, `macos.rs`, `linux.rs`.
- Add to `Cargo.toml`:
  - `global-hotkey = "0.8"`, `tray-icon = "0.24"`, `muda = "0.19"`, `tokio` (rt-multi-thread/net/io-util/macros), `arboard = "3"`, `enigo = "0.6"`
  - macOS only: `core-graphics = "0.25"`, `core-foundation = "0.10"`, `accessibility-sys = "0.2"`, `objc2 = "0.6"`
  - Linux only: `atspi = "0.30"`, `ashpd = "0.13"` (global_shortcuts feature)
- Verify `cargo check --all-targets` clean on macOS and Linux.

### Phase 9-B: macOS Daemon — DONE, verified on hardware 2026-06-19

- Register Ctrl+K with `global-hotkey` in `comboauth-daemon`.
- On trigger: query focused element via `accessibility-sys` for `kAXSecureTextFieldSubrole` / `kAXRoleAttribute`; classify as `FieldKind::{ Secure, Editable, Unknown }`.
- Open a minimal picker overlay (Ratatui floating terminal) showing matching combos.
- On user selection: write secret to clipboard via `arboard`, synthesize Cmd+V with `enigo`, clear clipboard after 200 ms.
- Check AX trust at startup via `AXIsProcessTrustedWithOptions`; exit with actionable error if missing.
- Socket path: `$TMPDIR/comboauth.sock`.
- IPC commands: `Status`, `Stop`, `ShowTui`.

### Phase 9-C: macOS Menu Bar Tray

- `comboauth-tray` creates a `tray-icon` menu bar item with icon + `muda` menu.
- Menu items: **Open ComboAuth** (spawns `comboauth` TUI), **Status** (queries daemon socket), **Stop Daemon**, **Quit**.
- Must run event loop on main thread (macOS requirement).
- On launch, check if daemon is running (connect to socket); if not, offer to start it.

### Phase 9-D: Linux X11

- `global-hotkey` handles X11 Ctrl+K registration; `enigo` handles synthetic paste.
- `atspi` detects `Role::PasswordText`; return `Unknown` aggressively when D-Bus/AT-SPI unavailable.
- Socket path: `$XDG_RUNTIME_DIR/comboauth/daemon.sock`.
- `tray-icon` on Linux requires AppIndicator/GTK at runtime — document this dependency.

### Phase 9-E: Linux Wayland Fallback

- Detect Wayland session (`WAYLAND_DISPLAY` env var).
- Use `ashpd` global shortcuts portal where supported; degrade otherwise.
- Fallback: copy secret to clipboard, send desktop notification ("Secret copied — paste manually").
- Document Wayland limitations clearly in README.

### Security Constraints

- Unix socket: `chmod 700` parent dir, `600` socket — same-UID only.
- Clipboard cleared after paste (200 ms minimum delay).
- Never log secrets or entry IDs to stdout/stderr.
- Accessibility permission shown once at daemon startup, never silently skipped.
- Ctrl+K hotkey is configurable in Settings (avoid permanently stealing it from all apps).

### Phase 9-F: Floating Combo Picker (macOS) — built, awaiting hardware verification

- Ctrl+K now opens a small floating NSPanel that briefly takes keyboard
  focus, captures the combo sequence via an NSEvent local monitor (no
  CGEventTap / Input Monitoring needed — same pattern as Spotlight/Alfred),
  matches it against persisted combo profiles, restores focus to the
  previously-frontmost app, and pastes the matched secret.
- Implementation: `src/picker/macos.rs`, dispatched from a background
  thread onto the main thread via `dispatch2::run_on_main`.
- Not yet covered: Linux equivalent (still logs field kind only).

### Done When

- `cargo build --release` produces all three binaries on macOS.
- Ctrl+K from any app triggers picker and pastes into TextEdit, Safari, and Terminal.
- Tray icon starts/stops daemon and launches TUI.
- Linux X11 path works end-to-end.
- Wayland degrades gracefully without panicking.

## 10. TUI Credential & Service Management

The Services/Combos screens let you create records, but several operations
on those records were never wired up — discovered 2026-06-22 while trying
to actually use the app end-to-end instead of just the demo data.

- [x] Edit a service's name (Services screen, `e`).
- [x] Delete a service, including its stored secret (Services screen, `d`,
      with `y`/`n` confirmation).
- [x] Delete a combo profile (Combos screen, `d`, with `y`/`n` confirmation).
      Any service assigned to the deleted combo is unassigned rather than
      left pointing at a dead profile id.
- [ ] **Set a service's secret from the TUI.** Currently there is no way to
      do this at all — `SecretStore::put_secret` is only ever called for
      the hardcoded demo data and in unit tests. Every service created
      through the TUI ends up `MissingSecret` forever unless you drop to
      the OS keychain directly (e.g. on macOS:
      `security add-generic-password -a "<service-id>" -s "comboauth" -w "<secret>" -U`).
      Needs a masked-input screen on the Services screen, same shape as the
      add/edit-name flow.
- [ ] Edit a combo profile's recorded sequence/timing (currently delete +
      re-record is the only way to change one).

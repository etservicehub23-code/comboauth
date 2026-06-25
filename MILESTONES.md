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

- `global-hotkey` handles X11 Ctrl+K registration; `enigo` handles synthetic paste. <!-- done: paste_and_clear/copy_and_clear in src/paste/linux.rs -->
- `atspi` detects `Role::PasswordText`; return `Unknown` aggressively when D-Bus/AT-SPI unavailable. <!-- done: src/focus/linux_atspi.rs, BFS walk with aggressive Unknown -->
- Socket path: `$XDG_RUNTIME_DIR/comboauth/daemon.sock`. <!-- done: src/ipc.rs socket_path() linux branch -->
- `tray-icon` on Linux requires AppIndicator/GTK at runtime — document this dependency. <!-- done: README.md Linux X11 section, distro table; tray implementation still pending -->

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
- [x] **Set a service's secret from the TUI.** Services screen, `s`: masked
      input (rendered as `•`), Enter writes it via `SecretStore::put_secret`,
      Esc cancels without storing. Input buffer is zeroized (not just
      cleared) on save and cancel, matching the zeroization bar the audit
      set for `SecretMaterial` itself.
- [x] Edit a combo profile's recorded sequence/timing (currently delete +
      re-record is the only way to change one). Combos screen, `e`: reuses
      the existing record-token-capture screen pre-filled with the profile's
      name, re-recording overwrites `sequence`/`gaps_ms` in place (same id,
      so service assignments survive) and only commits if persistence
      succeeds.

## 11. Paste Safety — Field-Kind Gating

`FieldKind::{Secure, Editable, NonEditable, Unknown}` (macOS AX-based focused
field detection, `src/focus/macos_ax.rs`) is computed on every Ctrl+K trigger
but currently only logged — paste happens into the focused field regardless
of its kind. Scoped 2026-06-23 from a codex-oracle review (read-only,
`docs/security/AUDIT.md`-style: not a security boundary against a malicious
app, but a real reduction in accidental-paste risk). Do NOT hard-block on
`!= Secure` — the oracle's matrix reasoning is that terminal password
prompts, custom widgets, and some Electron apps legitimately won't report
`Secure`, so a hard block would make the feature unusable in common cases.
Three-tier policy instead: `Secure` -> paste; `Editable`/`Unknown` -> require
an explicit extra confirmation before pasting; `NonEditable` -> refuse
auto-paste, offer clipboard-copy instead.

- [x] Add a shared gating helper (`focus::paste_decision(FieldKind) ->
      PasteDecision { AutoPaste, ConfirmFirst, Refuse }`) and wire it into the
      Ctrl+K picker path (`src/picker/macos.rs`). `ConfirmFirst` requires a
      second Enter press in the picker's existing key monitor before pasting
      (Esc still cancels; other keys are ignored while awaiting confirmation);
      which field kind triggered it is reported via eprintln only, since the
      panel has no text-rendering UI yet to show it on-screen — a known gap,
      not silent auto-paste. `Refuse` copies to clipboard via a new
      `paste::copy_and_clear` (same restore-after-delay logic as
      `paste_and_clear`, minus the keystroke) with an 8s clear delay (long
      enough to manually paste, unlike the near-instant clear used after an
      actual auto-paste) and reports why via eprintln. Daemon IPC
      `PasteSelected` path intentionally left unchanged (next checklist item).
- [x] Apply the same `paste_decision` policy to the IPC `PasteSelected`
      handler (`src/bin/comboauth-daemon.rs:163`, currently pastes
      unconditionally). This path doesn't have a `FieldKind` computed yet —
      add the `focused_field_kind()` call there too, same as
      `on_hotkey_triggered` already does, before deciding.
- [x] Update `docs/security/threat-model.md` to state plainly that AX field
      detection reduces *accidental* paste into the wrong field and is not a
      defense against a malicious/compromised focused app spoofing its
      accessibility role — it should not be described in a way that implies
      it's an authoritative security boundary. Advisory-only wording applied,
      table prefixed with 'AX-reported', clipboard fallback caveat added.

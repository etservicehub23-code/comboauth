# ComboAuth

ComboAuth is an experimental Rust TUI for exploring arcade-style combo input as a password workflow primitive.

The early product idea is simple: a user chooses a service, enters a memorable fighting-game-like combo, and ComboAuth maps that combo to a future secure action. This scaffold does not store real passwords, monitor global keyboard input, autofill fields, use the clipboard, or implement encryption. It is only the foundation for testing the interface, state model, and combo parsing logic safely.

## MVP Scope

- Rust 2024 application.
- Ratatui and Crossterm terminal interface.
- Central app state and event loop.
- Basic home screen with menu navigation.
- Minimal combo parser with unit tests.
- Demo/mock behavior only.

## Out of Scope for This Scaffold

- Real password storage.
- Encryption or key derivation.
- Global keyboard hooks.
- Browser, OS, or clipboard autofill.
- Cloud sync.
- Any production security claims.

## Run

```bash
cargo run
```

Press `q` or `Esc` to quit.

## Test

```bash
cargo test
```

## Direction

The safest path is to treat combo input as a user-interface layer first, not as the cryptographic secret itself. Later milestones should add threat modeling, OS keychain integration, and external security review before any real secrets are handled.
## Security Model

ComboAuth is not protection against malware, keyloggers, or a compromised machine.
It is a muscle-memory UX layer over the OS keychain — designed for convenience and shoulder-surfing resistance, not cryptographic security.

Secrets are stored in the native OS keychain (GNOME Keyring on Linux, Keychain on macOS).
Combo sequences are never written to disk. The audit log records activation events by service name only — no secret bytes, no combo input.
Clipboard content is automatically cleared after 10 seconds.

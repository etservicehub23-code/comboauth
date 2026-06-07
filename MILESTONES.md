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

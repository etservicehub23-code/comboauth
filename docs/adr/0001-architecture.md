# ADR 0001: OS Keychain Frontend Architecture

**Date:** 2026-06-12
**Status:** Accepted

## Context

ComboAuth needs to store and retrieve credentials. Three approaches were evaluated:

1. **Standalone vault** — ComboAuth manages its own encrypted blob on disk, derives a key from the combo, stores everything locally.
2. **KeePass plugin** — ComboAuth adds a combo-input front end to an existing KeePass-compatible database.
3. **OS keychain frontend** — ComboAuth gates access to credentials already stored in the OS keychain (Linux Secret Service / macOS Keychain) using a combo input UX.

## Decision

**Option 3: OS keychain frontend.**

ComboAuth is a UX layer, not a vault. Credentials are stored and encrypted by the OS keychain. ComboAuth adds:
- Combo-gated unlock flow
- Auto-relock on inactivity / screen change
- Clipboard-free credential display

## Reasoning

- The OS is already the security boundary for credentials on both Linux and macOS. Duplicating that boundary in application code introduces risk without benefit.
- A standalone vault requires ComboAuth to be correct about key derivation, encryption, and key erasure — none of which are simple. Mistakes here are catastrophic.
- The combo is a UX shortcut, not a cryptographic secret (see threat-model.md). Using it to derive an encryption key would provide weak security while giving users false confidence.
- A KeePass plugin ties ComboAuth to a third-party format and distribution chain. It also adds integration surface without providing stronger security than the OS keychain path.
- OS keychain APIs (Secret Service on Linux via `secret-service` crate, Keychain on macOS via `security-framework`) are well-audited and handle encryption at rest, session access control, and memory management.

## Consequences

- ComboAuth will expose a `SecretStore` trait with at minimum `get`, `set`, and `delete` operations.
- `src/vault/mock.rs` provides a `MockSecretStore` for testing and M4 behavior.
- `src/vault/os.rs` will provide `OsSecretStore` targeting Linux Secret Service first, then macOS Keychain behind the same trait.
- No real secrets are stored until M6 (after this review is accepted and the gate checklist is cleared).
- The README must clearly state that the security boundary is the OS keychain, not ComboAuth.

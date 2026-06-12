# ADR 0002: Combo Profile Storage

**Date:** 2026-06-12
**Status:** Accepted

## Context

A combo profile contains:
- Profile name
- Token sequence (e.g., `["Up", "Up", "Down", "Down"]`)
- Per-step timing gaps in milliseconds
- Service associations (which service this profile unlocks)

This is metadata, not the credential itself. The question is whether it can be stored as plain config or must be treated as sensitive.

## Decision

**Combo profiles are sensitive metadata and must be stored in the OS keychain or an encrypted local blob with the wrapping key in the OS keychain.**

They must not be stored as plaintext config files.

## Reasoning

Combo profiles reveal:
1. The exact sequence an attacker would need to replay to unlock credentials
2. Timing gaps, which reduce the search space further
3. Service associations, which leak what credentials the user holds

An attacker with read access to the filesystem who also has brief physical or remote access can replay the combo mechanically (automated input) without needing to observe the user. Plaintext profile storage converts a "watch the user type" attack into a "read a file" attack, which is strictly weaker.

Storing profiles in the OS keychain:
- Inherits the same session-level access control as the credentials themselves
- Means profile exfiltration requires the same privilege as credential exfiltration
- Keeps the attack surface consistent and auditable

If a separate encrypted blob is used (e.g., for bulk profile storage), the wrapping key must reside in the OS keychain, not on disk in plaintext.

## Consequences

- `ComboProfile` is treated as a secret item in the `SecretStore`, not as a plain config struct.
- Profile serialization format (e.g., JSON) is fine as long as the serialized bytes are stored encrypted.
- In `MockSecretStore`, profiles are stored in memory only — no disk write.
- In `OsSecretStore` (M6), profiles are stored as separate Secret Service / Keychain items, not alongside the credential they protect.
- The profile list (names only, no sequences/timing) may be shown in the TUI; the sequences and timing gaps are never displayed in plaintext after recording is complete.

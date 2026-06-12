# M6 Gate Checklist

All items must be satisfied before M6 (Controlled Secret Prototype) code can start.

## Documentation

- [x] `docs/security/threat-model.md` — threat model written and accepted
- [x] `docs/adr/0001-architecture.md` — OS keychain frontend decision recorded
- [x] `docs/adr/0002-profile-storage.md` — profile storage as sensitive metadata recorded
- [ ] README updated with explicit security limitations (malware/keylogger non-goals)

## Architecture

- [ ] `SecretStore` trait exists in `src/vault/mod.rs` with at minimum `get`, `set`, `delete`
- [ ] `MockSecretStore` in `src/vault/mock.rs` passes all existing tests
- [ ] `ComboProfile` extracted from `src/app.rs` into `src/profile.rs`
- [ ] `App` wired to use `SecretStore` trait (not concrete mock directly)

## Behavior Gates

- [ ] Unlock timeout: 15-second inactivity auto-relock implemented and tested
- [ ] Relock on screen change tested
- [ ] Relock on Esc tested
- [ ] Relock on quit tested
- [ ] Relock on failed match tested
- [ ] No real secrets stored anywhere in codebase (grep confirms)
- [ ] No plaintext profile serialization to disk anywhere in codebase

## CI / Test

- [ ] `cargo test` passes with no failures
- [ ] `cargo clippy` passes with no errors
- [ ] All vault-related tests run against `MockSecretStore` only

## Sign-off

- [ ] Dr. Torres accepts threat model
- [ ] Dr. Torres accepts ADR 0001 and ADR 0002
- [ ] M6 work begins only after all items above are checked

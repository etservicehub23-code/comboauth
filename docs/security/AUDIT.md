# ComboAuth Security Audit

Oracle: Codex gpt-5.5 (codex-oracle, high reasoning).
Baseline: 104 tests passing, 2 ignored (Secret Service integration).

---

## Phase 1 — Secret material lifecycle

### Findings

- [CRITICAL] `linux_oo7.rs:72` — `put_secret()` copies secret bytes into a plain `Vec<u8>` before passing to `oo7::Keyring::create_item`. The buffer is not zeroized when it drops. oo7 has `impl From<Zeroizing<Vec<u8>>> for Secret` making the fix zero-cost. — `src/vault/linux_oo7.rs:72`
- [HIGH] `MockSecretStore` carries `#[derive(Debug)]`, which would expose service-ID-keyed entries in any `format!("{:?}", store)` output. Although `SecretMaterial::Debug` is redacted, the store key layout is visible. — `src/vault/mock.rs:8`
- [HIGH] `App::with_persistence()` wires `secret_store: default_secret_store()` (mock secrets) even in the production persistence path. Acknowledged WIP per `src/main.rs:34-44`. — `src/app.rs:263`
- [INFO] `App` holds `secret_store: MockSecretStore` as a long-lived field that survives all screen changes. Secrets live until `App` drops. Acceptable for mock store; requires OS-store injection before production. — `src/app.rs:40`
- [INFO] `activate_quick_launch()` clones `SecretMaterial` from the store, delivers it, then drops the clone. The temporary is protected by `ZeroizeOnDrop`. The canonical copy in the store is intentionally retained. — `src/app.rs:537`
- [INFO] `SecretMaterial::Debug` is correctly redacted. TUI renders only step count (not tokens) for quick-launch and only service name for activation status. Combo sequences appear in cleartext in the test-lab screen, which is expected (they are key-token names, not secret bytes). — `src/ui.rs:108`, `src/ui.rs:287`, `src/ui.rs:257`

### Actions taken

- Fixed: `linux_oo7.rs:72` — changed `secret.expose_bytes().to_vec()` to `zeroize::Zeroizing::new(secret.expose_bytes().to_vec())`, which passes directly via oo7's `From<Zeroizing<Vec<u8>>>` impl. Transient credential buffer is now zeroized on drop.
- Fixed: `mock.rs:8` — removed `#[derive(Debug)]` from `MockSecretStore`; replaced with manual `Debug` impl that emits only `entries_count` and `..`.
- Accepted risk: Long-lived `MockSecretStore` in `App` — this is the mock/demo store; real production path will inject `Box<dyn SecretStore>` backed by the OS keyring (deferred WIP).
- Deferred: `App::with_persistence()` using mock secrets — tracked WIP in `main.rs`; out of scope for this audit cycle.

## Phase 2 — Persistence layer

### Findings

- [HIGH] `App::with_persistence()` swallows load errors via `.unwrap_or_default()`, causing a corrupted/tampered keychain to look like "first run" and overwrite the store with demo data — `src/app.rs:212-214`
- [MEDIUM] `ServiceRegistryDto` → `ServiceRegistry` used infallible `From`, ignoring `schema_version`; profiles validated schema but registry did not — `src/persistence.rs:122`
- [HIGH] `oo7::Keyring::new()` auto-selects an encrypted file backend when running sandboxed (Flatpak/portal), writing a keyring blob to `$XDG_DATA_HOME/keyrings/…` with temp `.tmpkeyring…` files. Not plaintext, but violates "never flat files" intent and leaks write-cadence metadata — `src/vault/linux_oo7.rs:40`
- [INFO] `put_item` passes `replace=true` to `create_item`, making overwrites idempotent by contract; backed by oo7's `remove_items` + push in the file backend and the D-Bus `replace` flag. Correct but untested by an integration test — `src/vault/linux_oo7.rs:118`
- [INFO] Save errors (save_profile, save_registry) are discarded with `let _ =` at `app.rs:653`, `712`, `738`. UI state can diverge from durable store on write failure.
- [INFO] macOS `macos_keychain.rs` implements `SecretStore` only; no `PersistenceStore` impl exists for macOS — `src/vault/macos_keychain.rs`

### Actions taken

- Fixed: `persistence.rs:122` — changed `From<ServiceRegistryDto>` to `TryFrom<ServiceRegistryDto>` with `schema_version` guard; added `service_registry_dto_rejects_wrong_schema_version` test.
- Fixed: `vault/linux_oo7.rs:load_registry()` — updated call site to `ServiceRegistry::try_from(dto)?` to propagate schema errors.
- Fixed: `app.rs:with_persistence()` — track `profiles_result.is_err() || registry_result.is_err()`; on error fall back to in-memory demo data without writing to the store, so recoverable keychain data is not overwritten.
- Accepted risk: oo7 file backend — ComboAuth is not expected to run inside Flatpak; non-sandboxed Linux uses D-Bus Secret Service only. Deferred to a dedicated sandboxing policy decision.
- Accepted risk: save-error discards — TUI cannot surface modal errors during keypress handlers; divergence is visible on next launch (reload shows old data). Deferred.
- Deferred: macOS `OsPersistenceStore` — tracked WIP; out of scope for this audit cycle.

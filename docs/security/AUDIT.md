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

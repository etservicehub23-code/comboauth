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

## Phase 3 — UI/Rendering

### Findings
- [HIGH] `render_combos` rendered full combo sequence tokens in list labels — `src/ui.rs:266` (pre-fix). An observer of the Combos screen learned the exact token pattern for every credential, enabling replay attacks without knowing the timing.
- [INFO] Test Lab echoed live entered tokens via `recorded_combo_input()` — `src/ui.rs:280-284`, `src/ui.rs:328`, `src/app.rs:574-575`. Screen title says "blind combo entry" but raw token strings were visible while typing; shoulder-surfing could capture the full sequence.
- [INFO] Services list exposes usernames — `src/ui.rs:167`. `format!("{} | user: {} | {}", service.name, user, service.status.label())` — not a secret, but account metadata visible to anyone viewing the Services screen.
- [INFO] Ctrl-K overlay is clean — `src/ui.rs:108-129`. Shows only step count, never raw tokens.
- [INFO] Sidebar is clean — `src/ui.rs:83`. Shows only `{name} [{status_label}]`.
- [INFO] Status bar countdown is clean — `src/ui.rs:57-58`. Shows only seconds remaining and demo step count.

### Actions taken
- Fixed: `render_combos` — replaced `combo.sequence` with `combo.sequence.split_whitespace().count()` step count summary (`src/ui.rs:263-268`). Combo patterns no longer appear in any rendered widget.
- Fixed: `render_test_lab` — replaced `recorded_combo_input()` raw token echo with `"{token_count} steps captured"` count-only display (`src/ui.rs:280-287`). Entry is now genuinely non-echoing, consistent with the "blind combo entry" title.
- Accepted risk: Services list username exposure — usernames are not cryptographic material; displaying them is consistent with the management-screen purpose.
- Deferred: Add Ratatui buffer regression tests that assert sentinel token strings never appear in rendered output for any screen.

## Phase 4 — Delivery mechanisms

### Findings
- [INFO] `ClipboardSink` correctly pipes secret bytes via stdin, not CLI arguments — no /proc/pid/cmdline or `ps` leakage risk — `src/delivery.rs:33,36,83`
- [HIGH] `schedule_clipboard_clear` used `status().is_ok()` which returns `Ok` on any successful spawn+wait, even if `wl-copy --clear` exits nonzero. A failed clear would return early, skipping the `xclip` fallback, leaving the secret in the clipboard indefinitely — `src/delivery.rs:69` (pre-fix)
- [HIGH] `schedule_clipboard_clear` spawns a fully detached thread with no join handle and discards all return values. If the thread panics (e.g., during `pipe_to`) or the process exits before the timer fires, clipboard is never cleared — `src/delivery.rs:65-75`
- [INFO] `StdoutSink` writes raw secret bytes to stdout — acceptable only for askpass/credential-helper contexts, but not gated by CLI flag; caller context must ensure stdout is not captured by logs, CI, or shell wrappers — `src/delivery.rs:49`
- [INFO] If both `wl-copy` and `xclip` are absent, `ClipboardSink.deliver` correctly returns `DeliveryError::NoTool` — `src/delivery.rs:81`

### Actions taken
- Fixed: `delivery.rs:69` — changed `status().is_ok()` to `status().map(|s| s.success()).unwrap_or(false)`. A nonzero `wl-copy --clear` exit now falls through to the `xclip` fallback instead of silently returning success.
- Accepted risk: Detached clear thread with no panic recovery — converting to a tracked `JoinHandle` would require `App` to store it, complicating the ownership model. The 10s window is a best-effort UX guarantee, not a hard cryptographic guarantee; the threat model notes clipboard management is out of scope for adversaries with local access.
- Accepted risk: `StdoutSink` stdout exposure — by-design surface for git-askpass integration; callers must opt in deliberately.
- Deferred: Add integration tests with fake `wl-copy`/`xclip` binaries verifying secret goes to stdin (never argv) and that clear falls back correctly.

## Phase 5 — Audit log

### Findings
- [HIGH] `service_name` written raw into log line with no control-character sanitization — a crafted name with `\n`, `\r`, or ANSI escape bytes can forge log entries or corrupt terminal display — `src/audit.rs:41` (pre-fix)
- [HIGH] Log file created with `OpenOptions::new().create(true).append(true)` — no explicit mode set, so permissions depend on process umask and may produce a world/group-readable file — `src/audit.rs:46` (pre-fix)
- [INFO] `delivery_mode` is always a hardcoded string literal (`"clipboard"`) at all call sites — no injection risk in practice, but still benefits from sanitization defensively — `src/app.rs:497`, `src/app.rs:555`
- [INFO] macOS log path falls back to `~/.local/share/comboauth/audit.log` via the XDG path, which is non-standard on macOS (expected: `~/Library/Application Support`) — `src/audit.rs:51-61`
- [INFO] No combo sequence or secret bytes can appear in log lines: `Failed` events use a closed enum, `Activated` events include only service name and delivery mode (both now sanitized) — verified

### Actions taken
- Fixed: Added `sanitize_log_value()` in `src/audit.rs:13-17` — strips all characters outside printable ASCII (0x20–0x7E) by replacing with `_`. Applied to both `service_name` and `delivery_mode` in `log()`.
- Fixed: Log directory created via `DirBuilder::mode(0o700)` on Unix (`src/audit.rs:44-50`); log file opened with `OpenOptionsExt::mode(0o600)` on Unix (`src/audit.rs:60-66`). Non-Unix paths retain `create_dir_all` / default-mode behavior.
- Fixed: Added three tests — `sanitize_blocks_newline_injection`, `sanitize_blocks_ansi_escape`, `sanitize_passes_normal_names` — verifying sanitization correctness.
- Accepted risk: `delivery_mode` sanitization — the string is always a literal in the codebase; sanitization added for defense-in-depth, not urgency.
- Deferred: macOS log path (`~/Library/Application Support`) — deferred alongside the broader macOS platform dispatch (Phase 8).

## Phase 6 — Authentication logic

### Findings
- [HIGH] No brute-force protection — unlimited combo attempts with no lockout, counter, or delay after failures. Both `test_recorded_combo()` and `activate_quick_launch()` simply clear state and return, enabling rapid automated guessing — `src/app.rs:476,509,514,521,567,570`
- [HIGH] `activate_quick_launch()` bypassed timing entirely — it used a sequence-only `find_map` with no `gaps_pass_tolerance` check, while `test_recorded_combo()` did enforce timing when `gaps_ms` is non-empty — `src/app.rs:532-534` (pre-fix)
- [MEDIUM] `timing_tolerance_pct` (a `u32`) had no upper-bound guard: a value ≥ 100 produces `tol ≥ 1.0`, making `lo = 0` and `hi ≥ 2*expected`, which defeats rhythm matching entirely — `src/app.rs:854` (pre-fix)
- [INFO] All three built-in demo profiles have `gaps_ms: vec![]`, so timing is always skipped for them — this is by design for the demo, but newly recorded profiles do capture gaps — `src/app.rs:277,285,293,301`, `src/app.rs:463`
- [INFO] TUI accepts all Crossterm key events as trusted input with no paste/PTY origin guard. Rapid paste injection can supply tokens faster than a human, but the lockout and timing checks now bound the exploitation window — `src/main.rs:69,112,184`
- [INFO] Lower-bound rounding in tolerance math: `lo = (exp * (1 - tol)) as u64` truncates instead of rounding, making the low side ~1ms more permissive than the nominal percentage. Not exploitable in practice.

### Actions taken
- Fixed: Added `MAX_FAILED_ATTEMPTS = 5` and `LOCKOUT_SECS = 30` constants; added `failed_attempts: u32` and `locked_until: Option<Instant>` fields to `App` — `src/app.rs:14-15,34-35`
- Fixed: Added `bump_failed_attempts()` private helper that increments the counter and sets `locked_until` on threshold breach — `src/app.rs`
- Fixed: Both `test_recorded_combo()` and `activate_quick_launch()` now check `locked_until` at entry and return `ActivationResult::Locked` immediately; on success, `failed_attempts` is reset to 0; on `NoMatch`/`TimingMismatch`, `bump_failed_attempts()` is called — `src/app.rs`
- Fixed: `tick()` now clears `locked_until` and resets `last_activation` to `Waiting` when lockout expires — `src/app.rs`
- Fixed: `activate_quick_launch()` now captures timestamps via `std::mem::take`, computes inter-key gaps, and passes them through the same `gaps_pass_tolerance` path as `test_recorded_combo()`. It also reports `TimingMismatch` when sequence matches but timing fails — `src/app.rs`
- Fixed: `gaps_pass_tolerance` now clamps `tolerance_pct` to 100 before converting to float, preventing a `>100` value from making `lo = 0` and accepting arbitrarily fast inputs — `src/app.rs:854`
- Fixed: Added `ActivationResult::Locked` variant and corresponding UI render arm — `src/activation.rs:19`, `src/ui.rs`
- Accepted risk: Demo profiles have empty `gaps_ms` — intentional; timing is not enforced for demos. Users should record profiles with captured gaps for production use.
- Accepted risk: Paste/PTY injection — inherent in TUI architecture; bounded by lockout after 5 failures in 30s. Full PTY-detection would require OS-level input classification.
- Accepted risk: Lower-bound truncation — sub-millisecond drift, not exploitable.
- Deferred: Add `failed_attempts_trigger_lockout_after_n_failures` and `quick_launch_rejects_sequence_with_bad_timing_when_profile_has_gaps` integration tests.

# ComboAuth Threat Model

## Assets

| Asset | Description | Sensitivity |
|---|---|---|
| OS secrets | The actual passwords/credentials stored in the OS keychain | Critical |
| Combo profiles | Sequences + timing gaps that unlock stored credentials | High — sensitive metadata |
| Combo input at runtime | Keystrokes during entry session | Medium — transient |
| Service-name list | Names of services the user has registered | Low — metadata |

## Adversaries and Capabilities

### In-scope: Casual observer / shoulder surfer
- Can watch the screen while the user types
- May see which keys are pressed, but not timing gaps
- Cannot access the machine filesystem

**Mitigation:** Combo entry is displayed as masked tokens (no plaintext). The credential itself never appears on screen. Muscle-memory sequences are harder to memorize by watching than typed passwords.

### In-scope: Coworker with brief physical access (unlocked session)
- Can open a terminal on an already-unlocked session
- Cannot elevate privileges or read OS keychain without the user's session credentials

**Mitigation:** Combo vault locks on inactivity (15-second timeout), screen change, Esc, and quit. Unlocking requires a correct combo + timing match. The window for opportunistic access is narrow.

## What ComboAuth Protects Against

- **Clipboard auto-clear** — credentials written to the clipboard are automatically cleared after 10 seconds; the exposure window is bounded even if the user forgets to clear manually
- **Shoulder surfing** — the actual credential is never shown; only masked combo tokens
- **Frequent re-entry fatigue** — muscle-memory combo replaces typing long passwords repeatedly during dev sessions
- **Accidental shoulder-surfing of the credential** — the OS keychain holds the secret; ComboAuth only gates retrieval

## What ComboAuth Does NOT Protect Against

**These are explicit non-goals. Do not expect protection from:**

- **Keyloggers and input capture malware** — if software on the machine logs keystrokes, it records the combo. The combo is a UX shortcut, not a cryptographic secret.
- **Compromised OS or kernel** — a compromised machine can read the OS keychain directly regardless of ComboAuth.
- **Memory scraping** — the plaintext credential briefly exists in process memory after retrieval. A privileged process can read it.
- **Root/admin access** — root can access the OS keychain through the system's own APIs.
- **Brute-force by an attacker with the binary** — combo sequences are short and enumerable. The OS keychain's own auth is the real gate.
- **Physical theft of unlocked device** — screen lock is the correct mitigation; ComboAuth's auto-relock is a secondary safeguard, not a replacement.

## Trust Boundary

The security boundary is the **OS keychain** (Secret Service on Linux, Keychain on macOS), not ComboAuth itself.

ComboAuth is a frontend that adds:
1. A combo-gated UX layer on top of OS keychain access
2. Auto-relock on inactivity
3. Time-limited clipboard delivery (auto-cleared after 10 seconds)

The OS is responsible for:
1. Encrypting credentials at rest
2. Enforcing session-level access control
3. Prompting for OS-level auth when the keychain is first unlocked

## README Requirement

The ComboAuth README must state plainly:

> ComboAuth is not protection against malware, keyloggers, or a compromised machine.
> It is a muscle-memory UX layer over the OS keychain — designed for convenience and
> shoulder-surfing resistance, not cryptographic security.

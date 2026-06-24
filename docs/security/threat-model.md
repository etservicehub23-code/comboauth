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

## AX Field-Kind Detection

On macOS, ComboAuth queries the accessibility API for the focused element's
role before pasting (`FieldKind::{Secure, Editable, NonEditable, Unknown}`).
The advisory policy, based only on what the focused app reports through AX
(`paste_decision`), is:

- AX-reported `Secure` → auto-paste without an extra confirmation
- AX-reported `Editable` / `Unknown` → require explicit confirmation before pasting
- AX-reported `NonEditable` → refuse auto-paste, offer clipboard-copy instead (usability
  fallback — clipboard is still an exposure channel, not a secure alternative)

`Secure` here means "the focused app reported a secure text field role via AX"; it does
not verify app identity, bundle ID, process integrity, domain, or whether the destination
logs received input.

**What this achieves:** reduces the risk of accidentally pasting a secret
into a chat window, browser address bar, or other visible text field when the
user triggers Ctrl+K on the wrong target.

**What this does NOT achieve:** this is not a security boundary. A focused
app controls its own accessibility attributes and can report any role it
chooses. A malicious or compromised application can trivially spoof
`kAXSecureTextFieldSubrole` to make itself look like a password field, or
report a legitimate role while logging every character received. The AX check
has no authority over a cooperative adversary.

Do not treat a `Secure` classification as proof that the destination is
trustworthy. The correct mental model is: AX gating prevents common accidents
(fat-finger, wrong window), not attacks by a focused app that controls its own
metadata.

## README Requirement

The ComboAuth README must state plainly:

> ComboAuth is not protection against malware, keyloggers, or a compromised machine.
> It is a muscle-memory UX layer over the OS keychain — designed for convenience and
> shoulder-surfing resistance, not cryptographic security.

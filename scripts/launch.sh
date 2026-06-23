#!/usr/bin/env bash
# Starts comboauth-daemon and comboauth-tray together, skipping either one
# if it's already running. macOS only (Linux daemon/tray are still stubs —
# see MILESTONES.md Phase 9-D/9-E).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="${COMBOAUTH_BIN_DIR:-$SCRIPT_DIR/../target/release}"

DAEMON_BIN="$BIN_DIR/comboauth-daemon"
TRAY_BIN="$BIN_DIR/comboauth-tray"
LOG_DIR="${COMBOAUTH_LOG_DIR:-/tmp}"

if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "comboauth: this launcher is macOS-only for now — Linux daemon/tray are still stubs (see MILESTONES.md)" >&2
    exit 1
fi

if [[ ! -x "$DAEMON_BIN" || ! -x "$TRAY_BIN" ]]; then
    echo "comboauth: binaries not found in $BIN_DIR" >&2
    echo "comboauth: run 'cargo build --release --features macos-keychain' first" >&2
    exit 1
fi

if pgrep -x "comboauth-daemon" >/dev/null 2>&1; then
    echo "comboauth: daemon already running, not starting a second instance"
else
    echo "comboauth: starting daemon..."
    nohup "$DAEMON_BIN" >"$LOG_DIR/comboauth-daemon.log" 2>&1 &
    disown
    # Give it a moment to finish its Accessibility check and register the
    # global hotkey before the tray tries to talk to it over the socket.
    sleep 1
fi

if pgrep -x "comboauth-tray" >/dev/null 2>&1; then
    echo "comboauth: tray already running, not starting a second instance"
else
    echo "comboauth: starting tray..."
    nohup "$TRAY_BIN" >"$LOG_DIR/comboauth-tray.log" 2>&1 &
    disown
fi

echo "comboauth: launched. Logs: $LOG_DIR/comboauth-daemon.log, $LOG_DIR/comboauth-tray.log"

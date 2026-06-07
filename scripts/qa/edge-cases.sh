#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
EVIDENCE="$ROOT/.omo/evidence/stp-panel-native-session-sidebar"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/stp-edge.XXXXXX")"
REGISTRY="$TMP_DIR/registry.json"
TERMINAL_ID="00000000-0000-0000-0000-000000000801"

cleanup() {
  set +e
  tmux -L stp-edge-test kill-server >/dev/null 2>&1
  rm -rf "$TMP_DIR"
  printf 'cleanup: killed tmux server stp-edge-test; removed %s\n' "$TMP_DIR"
}
trap cleanup EXIT

mkdir -p "$EVIDENCE"
cd "$ROOT"
cargo test -p stp-core malformed_registry_when_invalid_json -- --nocapture | tee "$EVIDENCE/C003-edge-malformed-registry.txt"
cargo test -p stp-core remove_stale_when_registry_contains_live_and_stale -- --nocapture | tee "$EVIDENCE/C003-edge-remove-stale.txt"
cargo test -p stp invalid_workspace_fails_when_terminal_command_runs -- --nocapture | tee "$EVIDENCE/C003-edge-invalid-workspace.txt"
cargo test -p stp-tmux missing_target_returns_error_when_session_absent -- --nocapture | tee "$EVIDENCE/C003-edge-missing-target.txt"
cargo test -p stp panel_sidebar_click_on_header_is_noop -- --nocapture | tee "$EVIDENCE/edge-invalid-click.txt"
cp "$EVIDENCE/C003-edge-missing-target.txt" "$EVIDENCE/edge-missing-session.txt"

if ./target/debug/stp open-cursor --registry "$TMP_DIR/registry.json" --terminal-id 00000000-0000-0000-0000-000000009999 > "$EVIDENCE/C003-edge-missing-terminal.txt" 2>&1; then
  printf 'missing terminal unexpectedly succeeded\n' >&2
  exit 1
fi
grep 'terminal not found' "$EVIDENCE/C003-edge-missing-terminal.txt"

if ./target/debug/stp terminate --registry "$REGISTRY" --terminal-id "$TERMINAL_ID" > "$EVIDENCE/C002-missing-yes.txt" 2>&1; then
  printf 'terminate without --yes unexpectedly succeeded\n' >&2
  exit 1
fi
grep 'refusing to terminate without --yes' "$EVIDENCE/C002-missing-yes.txt"

WORKSPACE="$TMP_DIR/worktree-already-exited"
mkdir -p "$WORKSPACE"
./target/debug/stp terminal --workspace "$WORKSPACE" --window-id 00000000-0000-0000-0000-000000000201 --terminal-id "$TERMINAL_ID" --socket stp-edge-test --registry "$REGISTRY" --shell sh --detach
tmux -L stp-edge-test kill-server >/dev/null 2>&1 || true
./target/debug/stp terminate --registry "$REGISTRY" --terminal-id "$TERMINAL_ID" --yes > "$EVIDENCE/C002-already-exited.txt"
grep "already exited $TERMINAL_ID" "$EVIDENCE/C002-already-exited.txt"

printf 'EDGE PASS\n'

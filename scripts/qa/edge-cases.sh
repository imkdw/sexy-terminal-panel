#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
EVIDENCE="$ROOT/.omo/evidence/terminal-panel"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/stp-edge.XXXXXX")"

cleanup() {
  set +e
  tmux -L stp-edge-test kill-server >/dev/null 2>&1
  rm -rf "$TMP_DIR"
  printf 'cleanup: killed tmux server stp-edge-test; removed %s\n' "$TMP_DIR"
}
trap cleanup EXIT

mkdir -p "$EVIDENCE"
cd "$ROOT"
cargo test -p stp-core malformed_registry_when_invalid_json -- --nocapture | tee "$EVIDENCE/task-3-malformed.txt"
cargo test -p stp-core remove_stale_when_registry_contains_live_and_stale -- --nocapture | tee "$EVIDENCE/task-9-stale.txt"
cargo test -p stp invalid_workspace_fails_when_terminal_command_runs -- --nocapture | tee "$EVIDENCE/task-5-error.txt"
cargo test -p stp-tmux missing_target_returns_error_when_session_absent -- --nocapture | tee "$EVIDENCE/task-4-error.txt"

if ./target/debug/stp open-code --registry "$TMP_DIR/registry.json" --terminal-id 00000000-0000-0000-0000-000000009999 > "$EVIDENCE/task-8-missing.txt" 2>&1; then
  printf 'missing terminal unexpectedly succeeded\n' >&2
  exit 1
fi
grep 'terminal not found' "$EVIDENCE/task-8-missing.txt"

printf 'EDGE PASS\n'

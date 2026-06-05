#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
EVIDENCE="$ROOT/.omo/evidence/terminal-panel"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/stp-release.XXXXXX")"

cleanup() {
  set +e
  tmux -L stp-release-open kill-server >/dev/null 2>&1
  rm -rf "$TMP_DIR"
  printf 'cleanup: killed tmux server stp-release-open; removed %s\n' "$TMP_DIR"
}
trap cleanup EXIT

mkdir -p "$EVIDENCE"
cd "$ROOT"
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
(cd extensions/vscode && bun install && bun test && bunx tsc --noEmit)
scripts/qa/check-loc.sh

if find crates extensions/vscode/src -type f | xargs grep -n 'xterm\|xterm.js' >/tmp/stp-no-browser-check.txt 2>/dev/null; then
  cat /tmp/stp-no-browser-check.txt
  printf 'browser terminal code found in MVP surface\n' >&2
  exit 1
fi
printf 'no browser/xterm MVP code\n'

WORKSPACE="$TMP_DIR/worktree-open"
mkdir -p "$WORKSPACE"
./target/debug/stp terminal --workspace "$WORKSPACE" --window-id 00000000-0000-0000-0000-000000000201 --terminal-id 00000000-0000-0000-0000-000000000401 --socket stp-release-open --registry "$TMP_DIR/registry.json" --shell sh --detach
./target/debug/stp open-code --registry "$TMP_DIR/registry.json" --terminal-id 00000000-0000-0000-0000-000000000401 --dry-run --log "$EVIDENCE/C003-open-code.log"
grep 'code --new-window' "$EVIDENCE/C003-open-code.log"

scripts/qa/e2e-local.sh
scripts/qa/edge-cases.sh
printf 'RELEASE PASS\n'

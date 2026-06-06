#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
EVIDENCE="$ROOT/.omo/evidence/stp-panel-session-sidebar"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/stp-e2e.XXXXXX")"
REGISTRY="$TMP_DIR/registry.json"
TERM_A="00000000-0000-0000-0000-000000000301"
TERM_B="00000000-0000-0000-0000-000000000302"
WINDOW_ID="00000000-0000-0000-0000-000000000201"
cleanup_done=0

cleanup() {
  set +e
  tmux -L stp-panel-qa kill-server >/dev/null 2>&1
  tmux -L stp-managed-qa kill-server >/dev/null 2>&1
  rm -rf "$TMP_DIR"
  cleanup_done=1
  printf 'cleanup: killed tmux servers stp-panel-qa/stp-managed-qa; removed %s\n' "$TMP_DIR"
}
trap cleanup EXIT

mkdir -p "$EVIDENCE"
cd "$ROOT"
cargo build -p stp >/dev/null
tmux -L stp-panel-qa kill-server >/dev/null 2>&1 || true
tmux -L stp-managed-qa kill-server >/dev/null 2>&1 || true

REPO="$TMP_DIR/repo"
mkdir -p "$REPO"
git -C "$REPO" init >/dev/null
git -C "$REPO" config user.email qa@example.test
git -C "$REPO" config user.name QA
printf 'seed\n' > "$REPO/README.md"
git -C "$REPO" add README.md
git -C "$REPO" commit -m seed >/dev/null
WT_A="$TMP_DIR/worktree-a"
WT_B="$TMP_DIR/worktree-b"
git -C "$REPO" worktree add "$WT_A" -b feature/a >/dev/null
git -C "$REPO" worktree add "$WT_B" -b feature/b >/dev/null

./target/debug/stp terminal --workspace "$WT_A" --window-id "$WINDOW_ID" --terminal-id "$TERM_A" --socket stp-managed-qa --registry "$REGISTRY" --shell sh --detach
./target/debug/stp terminal --workspace "$WT_B" --window-id "$WINDOW_ID" --terminal-id "$TERM_B" --socket stp-managed-qa --registry "$REGISTRY" --shell sh --detach

./target/debug/stp panel --registry "$REGISTRY" --layout 3x3 --once > "$EVIDENCE/C002-panel-initial.txt"
grep 'Layout: 3x3' "$EVIDENCE/C002-panel-initial.txt"
grep 'worktree-a' "$EVIDENCE/C002-panel-initial.txt"
grep 'worktree-b' "$EVIDENCE/C002-panel-initial.txt"

./target/debug/stp terminate --registry "$REGISTRY" --terminal-id "$TERM_B" --yes > "$EVIDENCE/C002-cli-terminate-b.txt"
grep "terminated $TERM_B" "$EVIDENCE/C002-cli-terminate-b.txt"
cp "$REGISTRY" "$EVIDENCE/C002-registry-after-terminate.json"
grep '"status": "exited"' "$EVIDENCE/C002-registry-after-terminate.json"
tmux -L stp-managed-qa has-session -t "stp-$TERM_A"
if tmux -L stp-managed-qa has-session -t "stp-$TERM_B" >/dev/null 2>&1; then
  printf 'terminated session B is still running\n' >&2
  exit 1
fi

./target/debug/stp qa-send-focused --registry "$REGISTRY" --terminal-id "$TERM_A" --text 'echo focused-ok'
sleep 0.5
./target/debug/stp qa-capture --registry "$REGISTRY" --terminal-id "$TERM_A" --lines 80 > "$EVIDENCE/C002-send-focused-target.txt"
grep 'focused-ok' "$EVIDENCE/C002-send-focused-target.txt"

tmux -L stp-managed-qa ls > "$EVIDENCE/C002-preserve.txt"
grep "stp-$TERM_A" "$EVIDENCE/C002-preserve.txt"
printf 'managed session A preserved after terminating B\n' > "$EVIDENCE/C002-panel-kill.txt"

cp "$REGISTRY" "$EVIDENCE/C002-registry-final.json"
printf 'E2E PASS\n'

if [ "$cleanup_done" -ne 0 ]; then
  printf 'cleanup already ran early\n' >&2
  exit 1
fi

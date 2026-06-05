#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
EVIDENCE="$ROOT/.omo/evidence/terminal-panel"
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

tmux -L stp-panel-qa new-session -d -s stp-panel-qa "cd '$ROOT' && ./target/debug/stp panel --registry '$REGISTRY' --layout 3x3"
sleep 0.5
tmux -L stp-panel-qa capture-pane -pt stp-panel-qa -S -200 > "$EVIDENCE/task-6-panel-initial.txt"
grep 'Layout: 3x3' "$EVIDENCE/task-6-panel-initial.txt"
grep 'worktree-a' "$EVIDENCE/task-6-panel-initial.txt"
if grep -q 'worktree-b' "$EVIDENCE/task-6-panel-initial.txt"; then
  printf 'panel showed worktree-b before registration\n' >&2
  exit 1
fi

./target/debug/stp terminal --workspace "$WT_B" --window-id "$WINDOW_ID" --terminal-id "$TERM_B" --socket stp-managed-qa --registry "$REGISTRY" --shell sh --detach
sleep 1
tmux -L stp-panel-qa capture-pane -pt stp-panel-qa -S -200 > "$EVIDENCE/task-6-panel-after-register.txt"
grep 'worktree-b' "$EVIDENCE/task-6-panel-after-register.txt"

tmux -L stp-panel-qa send-keys -t stp-panel-qa g
sleep 0.5
tmux -L stp-panel-qa capture-pane -pt stp-panel-qa -S -200 > "$EVIDENCE/task-6-toggle.txt"
grep 'Layout: 2x2' "$EVIDENCE/task-6-toggle.txt"
grep 'worktree-a' "$EVIDENCE/task-6-toggle.txt"
printf 'layout toggle preserves focus terminal id %s\n' "$TERM_A"

./target/debug/stp qa-send-focused --registry "$REGISTRY" --terminal-id "$TERM_A" --text 'echo focused-ok'
sleep 0.5
./target/debug/stp qa-capture --registry "$REGISTRY" --terminal-id "$TERM_A" --lines 80 > "$EVIDENCE/task-9-send-focused-target.txt"
./target/debug/stp qa-capture --registry "$REGISTRY" --terminal-id "$TERM_B" --lines 80 > "$EVIDENCE/task-9-send-focused-other.txt"
grep 'focused-ok' "$EVIDENCE/task-9-send-focused-target.txt"
if grep -q 'focused-ok' "$EVIDENCE/task-9-send-focused-other.txt"; then
  printf 'focused input reached non-target pane\n' >&2
  exit 1
fi

tmux -L stp-panel-qa send-keys -t stp-panel-qa q
sleep 0.5
if tmux -L stp-panel-qa ls >/dev/null 2>&1; then
  printf 'panel tmux server still running after q\n' >&2
  exit 1
fi
tmux -L stp-managed-qa ls > "$EVIDENCE/task-6-preserve.txt"
grep "stp-$TERM_A" "$EVIDENCE/task-6-preserve.txt"
printf 'managed sessions preserved after panel quit\n'

cp "$REGISTRY" "$EVIDENCE/task-11-registry.json"
printf 'E2E PASS\n'

if [ "$cleanup_done" -ne 0 ]; then
  printf 'cleanup already ran early\n' >&2
  exit 1
fi

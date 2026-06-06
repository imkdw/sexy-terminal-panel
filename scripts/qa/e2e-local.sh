#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
EVIDENCE="$ROOT/.omo/evidence/stp-panel-native-session-sidebar"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/stp-e2e.XXXXXX")"
REGISTRY="$TMP_DIR/registry.json"
TERM_A="00000000-0000-0000-0000-000000000301"
TERM_B="00000000-0000-0000-0000-000000000302"
TERM_C="00000000-0000-0000-0000-000000000303"
TERM_D="00000000-0000-0000-0000-000000000304"
TERM_E="00000000-0000-0000-0000-000000000305"
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
WT_C="$TMP_DIR/worktree-c"
WT_D="$TMP_DIR/worktree-d"
WT_E="$TMP_DIR/worktree-e"
git -C "$REPO" worktree add "$WT_A" -b feature/a >/dev/null
git -C "$REPO" worktree add "$WT_B" -b feature/b >/dev/null
git -C "$REPO" worktree add "$WT_C" -b feature/c >/dev/null
git -C "$REPO" worktree add "$WT_D" -b feature/d >/dev/null
git -C "$REPO" worktree add "$WT_E" -b feature/e >/dev/null

./target/debug/stp terminal --workspace "$WT_A" --window-id "$WINDOW_ID" --terminal-id "$TERM_A" --socket stp-managed-qa --registry "$REGISTRY" --shell sh --detach
./target/debug/stp terminal --workspace "$WT_B" --window-id "$WINDOW_ID" --terminal-id "$TERM_B" --socket stp-managed-qa --registry "$REGISTRY" --shell sh --detach
./target/debug/stp terminal --workspace "$WT_C" --window-id "$WINDOW_ID" --terminal-id "$TERM_C" --socket stp-managed-qa --registry "$REGISTRY" --shell sh --detach
./target/debug/stp terminal --workspace "$WT_D" --window-id "$WINDOW_ID" --terminal-id "$TERM_D" --socket stp-managed-qa --registry "$REGISTRY" --shell sh --detach
./target/debug/stp terminal --workspace "$WT_E" --window-id "$WINDOW_ID" --terminal-id "$TERM_E" --socket stp-managed-qa --registry "$REGISTRY" --shell sh --detach

./target/debug/stp panel --registry "$REGISTRY" --layout 3x3 --once > "$EVIDENCE/C002-panel-initial.txt"
grep 'Layout: 3x3' "$EVIDENCE/C002-panel-initial.txt"
grep 'worktree-a' "$EVIDENCE/C002-panel-initial.txt"
grep 'worktree-b' "$EVIDENCE/C002-panel-initial.txt"

printf -v PANEL_COMMAND 'STP_TMUX_SOCKET=%q %q panel --registry %q --layout 2x2' \
  stp-managed-qa "$ROOT/target/debug/stp" "$REGISTRY"
tmux -L stp-panel-qa new-session -d -s stp-panel-wrapper "$PANEL_COMMAND"
for _ in {1..30}; do
  if tmux -L stp-managed-qa list-panes -t stp-panel -F '#{pane_id}:#{@stp-pane-key}:#{pane_active}' > "$EVIDENCE/C002-native-panel-panes-initial.txt" 2>/dev/null; then
    break
  fi
  sleep 0.2
done
grep 'stp-sidebar' "$EVIDENCE/C002-native-panel-panes-initial.txt"
grep "$TERM_A" "$EVIDENCE/C002-native-panel-panes-initial.txt"
grep "$TERM_B" "$EVIDENCE/C002-native-panel-panes-initial.txt"

SIDEBAR_PANE="$(tmux -L stp-managed-qa list-panes -t stp-panel -F '#{pane_id}:#{@stp-pane-key}' | awk -F: '$2 == "stp-sidebar" { print $1; exit }')"
tmux -L stp-managed-qa capture-pane -p -t "$SIDEBAR_PANE" -S -20 > "$EVIDENCE/C002-native-sidebar.txt"
grep 'STP sessions' "$EVIDENCE/C002-native-sidebar.txt"
grep 'worktree-e' "$EVIDENCE/C002-native-sidebar.txt"
cat "$EVIDENCE/C002-native-panel-panes-initial.txt" "$EVIDENCE/C002-native-sidebar.txt" > "$EVIDENCE/e2e-sidebar-layout.txt"

./target/debug/stp panel-select --registry "$REGISTRY" --socket stp-managed-qa --mouse-line "2 00000000 worktree-b feature/b"
tmux -L stp-managed-qa list-panes -t stp-panel -F '#{@stp-pane-key}:#{pane_active}' > "$EVIDENCE/C002-native-select-visible.txt"
grep "$TERM_B:1" "$EVIDENCE/C002-native-select-visible.txt"
cp "$EVIDENCE/C002-native-select-visible.txt" "$EVIDENCE/e2e-click-focus.txt"

ACTIVE_BEFORE_INVALID="$(tmux -L stp-managed-qa list-panes -t stp-panel -F '#{@stp-pane-key}:#{pane_active}' | awk -F: '$2 == "1" { print $1; exit }')"
./target/debug/stp panel-select --registry "$REGISTRY" --socket stp-managed-qa --mouse-line 'STP sessions'
ACTIVE_AFTER_INVALID="$(tmux -L stp-managed-qa list-panes -t stp-panel -F '#{@stp-pane-key}:#{pane_active}' | awk -F: '$2 == "1" { print $1; exit }')"
printf '%s\n%s\n' "$ACTIVE_BEFORE_INVALID" "$ACTIVE_AFTER_INVALID" > "$EVIDENCE/C002-native-invalid-click.txt"
test "$ACTIVE_BEFORE_INVALID" = "$ACTIVE_AFTER_INVALID"

./target/debug/stp panel-select --registry "$REGISTRY" --socket stp-managed-qa --mouse-line "5 00000000 worktree-e feature/e"
tmux -L stp-managed-qa list-panes -t stp-panel -F '#{@stp-pane-key}:#{pane_active}' > "$EVIDENCE/C002-native-select-overflow.txt"
grep "$TERM_E:1" "$EVIDENCE/C002-native-select-overflow.txt"
cp "$EVIDENCE/C002-native-select-overflow.txt" "$EVIDENCE/e2e-click-overflow.txt"

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

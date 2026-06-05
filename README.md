# Sexy Terminal Panel

Sexy Terminal Panel is a macOS-first local tool for keeping VS Code worktree terminals in tmux and managing them from one native terminal panel.

## MVP Scope

- `stp terminal` registers a workspace terminal and starts or attaches a tmux session.
- `stp panel` shows managed terminals in a native terminal grid.
- The default panel layout is `2x2`; pass `--layout 3x3` when you need the larger grid.
- `h/j/k/l`, arrows, and `Tab` move panel focus.
- `stp qa-send-focused` and panel actions route by terminal id, not visual index.
- `stp open-code` opens the registered workspace through `code --new-window` or records the deterministic fallback in dry-run mode.
- Browser/xterm.js terminal control is intentionally not part of the MVP.

## Install

Dry-run the local installer:

```sh
scripts/install-local.sh --dry-run
```

Install to a custom prefix:

```sh
scripts/install-local.sh --prefix "$HOME/.local"
```

The VS Code extension contributes the `STP: tmux` terminal profile and sets it
as the default integrated terminal profile on macOS. It runs:

```text
stp terminal --workspace <path> --window-id <id> --terminal-id <id>
```

If VS Code cannot find `stp` when launched outside a shell, set:

```json
"stp.binaryPath": "/absolute/path/to/stp"
```

## State

Registry path:

```text
${XDG_STATE_HOME:-~/.local/state}/sexy-terminal-panel/registry.json
```

Managed tmux socket default:

```text
stp-managed
```

## Keymap

- `q`: quit panel only
- `g`: toggle `3x3`/`2x2`
- `h/j/k/l` and arrow keys: move focus
- `Tab`: next cell

## Verification

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cd extensions/vscode && bun test && bunx tsc --noEmit
scripts/qa/e2e-local.sh
scripts/qa/edge-cases.sh
scripts/qa/release-gate.sh
```

The QA scripts create disposable git worktrees and isolated tmux servers. Cleanup receipts are printed before exit.

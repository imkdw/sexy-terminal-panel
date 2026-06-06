# Sexy Terminal Panel

Sexy Terminal Panel is a macOS-first local tool for keeping VS Code worktree terminals in tmux and managing them from one native terminal panel.

## MVP Scope

- `stp terminal` registers a workspace terminal and starts or attaches a tmux session.
- `stp panel` shows managed terminals in a native terminal grid.
- The VS Code extension adds an Explorer `STP Terminals` view that lists tracked
  STP integrated terminals and live registry sessions.
- Clicking an `STP Terminals` item shows the matching integrated terminal or
  opens one that attaches to the selected registry session.
- `stp terminate --terminal-id <id> --yes` terminates one registered tmux-backed
  terminal and marks its registry entry as `exited`.
- `stp registry cleanup-zombies --yes` removes registry entries whose tmux
  sessions are already gone.
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

Tracked STP terminals appear in Explorer under `STP Terminals`. Select an item
there to reveal that integrated terminal, or to create an integrated terminal
that attaches to the selected live registry session. The extension runs zombie
cleanup on activation and when a tracked STP terminal closes, so disconnected
tmux sessions do not stay in the sidebar. With an STP terminal focused,
`cmd+shift+backspace` runs `stp terminate --terminal-id <id> --yes`; the
extension disposes the integrated terminal only after the CLI termination
succeeds. Non-STP terminals are ignored.

## State

Registry path:

```text
${XDG_STATE_HOME:-~/.local/state}/sexy-terminal-panel/registry.json
```

Set `stp.registryPath` when VS Code should use a non-default registry file.

Managed tmux socket default:

```text
stp-managed
```

## Keymap

- `q`: quit panel only
- `g`: toggle `3x3`/`2x2`
- `h/j/k/l` and arrow keys: move focus
- `Tab`: next cell
- `prefix K`: confirm and terminate the selected managed terminal from
  `stp panel`
- VS Code `cmd+shift+backspace`: terminate the focused tracked STP terminal

The native panel uses `prefix K`, not raw `K`, so uppercase input still reaches
attached terminals.

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

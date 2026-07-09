# Sexy Terminal Panel

Sexy Terminal Panel is a macOS-first local tool for keeping Cursor worktree terminals in tmux and managing them from one native terminal panel.

## MVP Scope

- `stp terminal` registers a workspace terminal and starts or attaches a tmux session.
- `stp panel` shows a native left session list next to the right content grid
  of managed terminal panes.
- `stp tui` is an alias for `stp panel` when you want the terminal UI entrypoint.
- `stp panel` and `stp panel --once` remove live registry entries whose tmux
  sessions are already gone before rendering.
- The Cursor extension adds an Explorer `STP Terminals` view that lists tracked
  STP integrated terminals and live registry sessions.
- Closing an STP integrated terminal marks its registry entry as `detached`, so
  `stp panel` only shows STP terminals that are still open in Cursor.
- Clicking an `STP Terminals` item shows the matching integrated terminal or
  opens one that attaches to the selected registry session.
- `stp terminate --terminal-id <id> --yes` terminates one registered tmux-backed
  terminal and marks its registry entry as `exited`.
- `stp registry cleanup-zombies --yes` removes registry entries whose tmux
  sessions are already gone.
- The default panel layout is `2x2`; pass `--layout 3x3` when you need the larger grid.
- Click a session in the native panel's left list to focus its existing right
  pane. If that session is not visible, the panel opens it in the first empty
  right pane, or replaces the rightmost content pane when the grid is full.
- `stp qa-send-focused` and panel actions route by terminal id, not visual index.
- `stp open-cursor` opens the registered workspace through `cursor --new-window` or records the deterministic fallback in dry-run mode.
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

The Cursor extension contributes the `STP: tmux` terminal profile and sets it
as the default integrated terminal profile on macOS. It runs:

```text
stp terminal --workspace <path> --window-id <id> --terminal-id <id>
```

If Cursor cannot find `stp` when launched outside a shell, set:

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

Set `stp.registryPath` when Cursor should use a non-default registry file.

Managed tmux socket default:

```text
stp-managed
```

## Keymap

- `q`: quit panel only
- sidebar shows live session count, click action, and the main panel keys
- left session list click: focus that session, or open it in the first empty
  right-side pane; when the grid is full, replace the rightmost content pane
- `prefix K`: confirm and terminate the selected managed terminal from
  `stp panel`
- Cursor `cmd+shift+backspace`: terminate the focused tracked STP terminal

The native panel uses `prefix K`, not raw `K`, so uppercase input still reaches
attached terminals.

## Verification

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cd extensions/cursor && bun test && bunx tsc --noEmit
scripts/qa/e2e-local.sh
scripts/qa/edge-cases.sh
scripts/qa/release-gate.sh
```

The QA scripts create disposable git worktrees and isolated tmux servers. Cleanup receipts are printed before exit.

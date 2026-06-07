import { mkdtempSync, writeFileSync } from "node:fs"
import { tmpdir } from "node:os"
import { join } from "node:path"

import { describe, expect, test } from "bun:test"

import { loadLiveRegistrySessions, selectRegistryPath } from "../src/stpRegistry"

describe("stpRegistry", () => {
  test("loads only live sessions from the registry file", () => {
    const dir = mkdtempSync(join(tmpdir(), "stp-registry-test-"))
    const registryPath = join(dir, "registry.json")
    writeFileSync(
      registryPath,
      JSON.stringify({
        terminals: [
          {
            terminal_id: "00000000-0000-0000-0000-000000000101",
            workspace_path: "/tmp/worktree-a",
            tmux_socket: "stp-managed",
            tmux_session: "stp-00000000-0000-0000-0000-000000000101",
            status: "live",
          },
          {
            terminal_id: "00000000-0000-0000-0000-000000000102",
            workspace_path: "/tmp/worktree-b",
            tmux_socket: "stp-managed",
            tmux_session: "stp-00000000-0000-0000-0000-000000000102",
            status: "detached",
          },
          {
            terminal_id: "00000000-0000-0000-0000-000000000103",
            workspace_path: "/tmp/worktree-c",
            tmux_socket: "stp-managed",
            tmux_session: "stp-00000000-0000-0000-0000-000000000103",
            status: "stale",
          },
        ],
      }),
    )

    expect(loadLiveRegistrySessions(registryPath)).toEqual([
      {
        name: "STP: worktree-a 00000000",
        terminalId: "00000000-0000-0000-0000-000000000101",
        workspacePath: "/tmp/worktree-a",
        tmuxSocket: "stp-managed",
        tmuxSession: "stp-00000000-0000-0000-0000-000000000101",
        status: "live",
      },
    ])
  })

  test("uses XDG state home for the default registry path", () => {
    expect(
      selectRegistryPath(
        { defaultValue: "" },
        { HOME: "/Users/example", XDG_STATE_HOME: "/tmp/state" },
      ),
    ).toBe("/tmp/state/sexy-terminal-panel/registry.json")
  })
})

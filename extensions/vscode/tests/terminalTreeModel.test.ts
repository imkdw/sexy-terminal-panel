import { describe, expect, test } from "bun:test"

import { mergeTerminalTreeItems } from "../src/terminalTreeModel"

describe("terminalTreeModel", () => {
  test("merges opened sessions before registry-only sessions without duplicates", () => {
    const opened = {
      terminalId: "00000000-0000-0000-0000-000000000101",
      name: "STP: worktree-a 00000000",
      workspacePath: "/tmp/worktree-a",
      terminal: "terminal-a",
    }
    const registryOnly = {
      terminalId: "00000000-0000-0000-0000-000000000102",
      name: "STP: worktree-b 00000000",
      workspacePath: "/tmp/worktree-b",
      tmuxSocket: "stp-managed",
      tmuxSession: "stp-00000000-0000-0000-0000-000000000102",
      status: "live" as const,
    }

    expect(
      mergeTerminalTreeItems(
        [opened],
        [
          {
            ...opened,
            tmuxSocket: "stp-managed",
            tmuxSession: "stp-00000000-0000-0000-0000-000000000101",
            status: "live",
          },
          registryOnly,
        ],
      ),
    ).toEqual([
      { kind: "opened", session: opened },
      { kind: "registry", session: registryOnly },
    ])
  })
})

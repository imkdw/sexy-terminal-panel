import { describe, expect, test } from "bun:test"
import packageJson from "../package.json"
import { buildTerminalArgs, selectBinaryPath, selectWorkspacePath } from "../src/terminalProfile"

describe("terminalProfile", () => {
  test("builds stp terminal args when workspace is available", () => {
    const args = buildTerminalArgs({
      binaryPath: "/opt/stp/bin/stp",
      workspacePath: "/tmp/worktree-a",
      windowId: "00000000-0000-0000-0000-000000000001",
      terminalId: "00000000-0000-0000-0000-000000000101",
      shellPath: "zsh",
    })

    expect(args).toEqual([
      "/opt/stp/bin/stp",
      "terminal",
      "--workspace",
      "/tmp/worktree-a",
      "--window-id",
      "00000000-0000-0000-0000-000000000001",
      "--terminal-id",
      "00000000-0000-0000-0000-000000000101",
      "--shell",
      "zsh",
    ])
  })

  test("contributes STP tmux terminal profile", () => {
    expect(packageJson.contributes.terminal.profiles).toContainEqual({
      id: "stp.tmuxTerminal",
      title: "STP: tmux",
    })
  })

  test("defaults macOS integrated terminals to STP tmux", () => {
    expect(packageJson.contributes.configurationDefaults).toEqual({
      "terminal.integrated.defaultProfile.osx": "STP: tmux",
    })
  })

  test("keeps stp binary path machine scoped", () => {
    expect(packageJson.contributes.configuration.properties["stp.binaryPath"].scope).toBe(
      "machine",
    )
  })

  test("uses only global or default stp binary path", () => {
    expect(
      selectBinaryPath({
        defaultValue: "stp",
        globalValue: "/Users/example/.local/bin/stp",
      }),
    ).toBe("/Users/example/.local/bin/stp")
    expect(selectBinaryPath({ defaultValue: "stp" })).toBe("stp")
    expect(selectBinaryPath(undefined)).toBe("stp")
  })

  test("returns user-visible no workspace error when no folder is open", () => {
    expect(() => selectWorkspacePath([])).toThrow("No workspace folder is open")
  })
})

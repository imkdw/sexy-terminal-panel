import { describe, expect, test } from "bun:test"
import packageJson from "../package.json"
import {
  buildTerminalArgs,
  selectBinaryPath,
  selectTmuxSocket,
  selectWorkspacePath,
} from "../src/terminalProfile"
import { selectRegistryPath } from "../src/stpRegistry"

describe("terminalProfile", () => {
  test("builds stp terminal args when workspace is available", () => {
    const args = buildTerminalArgs({
      binaryPath: "/opt/stp/bin/stp",
      workspacePath: "/tmp/worktree-a",
      windowId: "00000000-0000-0000-0000-000000000001",
      terminalId: "00000000-0000-0000-0000-000000000101",
      tmuxSocket: "stp-workspace-a",
      registryPath: "/tmp/stp-registry.json",
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
      "--socket",
      "stp-workspace-a",
      "--registry",
      "/tmp/stp-registry.json",
      "--shell",
      "zsh",
    ])
  })

  test("contributes STP tmux terminal profile", () => {
    expect(packageJson.extensionKind).toEqual(["ui"])
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

  test("keeps stp tmux socket machine scoped", () => {
    expect(packageJson.contributes.configuration.properties["stp.tmuxSocket"].scope).toBe(
      "machine",
    )
  })

  test("keeps stp registry path machine scoped", () => {
    expect(packageJson.contributes.configuration.properties["stp.registryPath"].scope).toBe(
      "machine",
    )
  })

  test("contributes session list view and terminate shortcut", () => {
    expect(packageJson.contributes.views.explorer).toContainEqual({
      id: "stp.terminals",
      name: "STP Terminals",
    })
    expect(
      packageJson.contributes.commands.some(
        (command) =>
          command.command === "stp.newTerminal" &&
          command.title === "New STP Terminal" &&
          command.category === "STP",
      ),
    ).toBe(true)
    expect(
      packageJson.contributes.commands.some(
        (command) =>
          command.command === "stp.showTerminal" &&
          command.title === "Show STP Terminal" &&
          command.category === "STP",
      ),
    ).toBe(true)
    expect(
      packageJson.contributes.commands.some(
        (command) =>
          command.command === "stp.terminateCurrentTerminal" &&
          command.title === "Terminate Current Terminal" &&
          command.category === "STP",
      ),
    ).toBe(true)
    expect(packageJson.contributes.keybindings).toContainEqual({
      command: "stp.terminateCurrentTerminal",
      key: "cmd+shift+backspace",
      mac: "cmd+shift+backspace",
      when: "terminalFocus",
    })
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

  test("uses only global or default stp tmux socket", () => {
    expect(
      selectTmuxSocket({
        defaultValue: "stp-managed",
        globalValue: "stp-workspace-a",
      }),
    ).toBe("stp-workspace-a")
    expect(selectTmuxSocket({ defaultValue: "stp-managed" })).toBe("stp-managed")
    expect(selectTmuxSocket(undefined)).toBe("stp-managed")
  })

  test("uses configured or default stp registry path", () => {
    expect(
      selectRegistryPath({
        defaultValue: "",
        globalValue: "/tmp/custom-registry.json",
      }),
    ).toBe("/tmp/custom-registry.json")
    expect(
      selectRegistryPath(
        { defaultValue: "" },
        { HOME: "/Users/example", XDG_STATE_HOME: "/tmp/state" },
      ),
    ).toBe("/tmp/state/sexy-terminal-panel/registry.json")
  })

  test("returns user-visible no workspace error when no folder is open", () => {
    expect(() => selectWorkspacePath([])).toThrow("No workspace folder is open")
  })
})

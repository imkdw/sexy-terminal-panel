import { describe, expect, test } from "bun:test"
import packageJson from "../package.json"
import {
  TerminalSessionStore,
  buildTerminalSessionName,
  isStpTerminalName,
  shortTerminalId,
} from "../src/terminalSessions"

describe("terminalSessions", () => {
  test("builds readable terminal names from the workspace and terminal id", () => {
    const name = buildTerminalSessionName({
      workspacePath: "/Users/example/src/worktree-a",
      terminalId: "00000000-0000-0000-0000-000000000101",
    })

    expect(name).toBe("STP: worktree-a 00000000")
    expect(isStpTerminalName(name)).toBe(true)
    expect(isStpTerminalName("STP: worktree-a 0101")).toBe(false)
    expect(shortTerminalId("00000000-0000-0000-0000-000000000101")).toBe("00000000")
  })

  test("tracks pending and opened terminals by session id", () => {
    const store = new TerminalSessionStore<string>()
    const session = store.createPending({
      terminalId: "00000000-0000-0000-0000-000000000101",
      workspacePath: "/tmp/worktree-a",
    })

    const opened = store.attachOpenedTerminal({
      name: session.name,
      terminal: "terminal-a",
    })

    expect(opened?.terminalId).toBe("00000000-0000-0000-0000-000000000101")
    expect(store.sessions()).toEqual([
      {
        terminalId: "00000000-0000-0000-0000-000000000101",
        name: "STP: worktree-a 00000000",
        workspacePath: "/tmp/worktree-a",
        terminal: "terminal-a",
      },
    ])
  })

  test("tracks opened terminals by their original creation name", () => {
    const store = new TerminalSessionStore<string>()
    const session = store.createPending({
      terminalId: "00000000-0000-0000-0000-000000000101",
      workspacePath: "/tmp/worktree-a",
    })

    const opened = store.attachOpenedTerminal({
      initialName: session.name,
      name: "tmux",
      terminal: "terminal-a",
    })

    expect(opened?.name).toBe("STP: worktree-a 00000000")
    expect(store.sessionForTerminal("terminal-a")?.terminalId).toBe(
      "00000000-0000-0000-0000-000000000101",
    )
    expect(
      store.attachOpenedTerminal({
        initialName: session.name,
        name: "tmux",
        terminal: "terminal-b",
      }),
    ).toBeUndefined()
  })

  test("does not terminate untracked terminals", () => {
    const store = new TerminalSessionStore<string>()
    const session = store.createPending({
      terminalId: "00000000-0000-0000-0000-000000000101",
      workspacePath: "/tmp/worktree-a",
    })
    store.attachOpenedTerminal({
      name: session.name,
      terminal: "terminal-a",
    })

    expect(store.sessionForTerminal("terminal-b")).toBeUndefined()
    expect(store.sessionForTerminal("terminal-a")?.terminalId).toBe(
      "00000000-0000-0000-0000-000000000101",
    )
  })

  test("contributes a left-side STP terminal list", () => {
    expect(packageJson.contributes.views.explorer).toContainEqual({
      id: "stp.terminals",
      name: "STP Terminals",
    })
  })

  test("contributes a keyboard shortcut for terminating the current STP terminal", () => {
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

  test("keeps tree-only show command out of the command palette", () => {
    expect(packageJson.contributes.menus.commandPalette).toContainEqual({
      command: "stp.showTerminal",
      when: "false",
    })
  })
})

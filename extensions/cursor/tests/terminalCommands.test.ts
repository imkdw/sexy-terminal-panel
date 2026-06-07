import { describe, expect, test } from "bun:test"
import {
  buildCleanupZombiesArgs,
  buildTerminateArgs,
  cleanupZombieSessions,
  showTrackedTerminal,
  terminateCurrentTerminal,
  type CommandRunner,
  type MessageSink,
} from "../src/terminalCommands"
import { TerminalSessionStore } from "../src/terminalSessions"

describe("terminalCommands", () => {
  test("shows tracked terminal without stealing focus", () => {
    const terminal = new FakeTerminal("terminal-a")

    showTrackedTerminal({
      terminalId: "00000000-0000-0000-0000-000000000101",
      name: "STP: worktree-a 00000000",
      workspacePath: "/tmp/worktree-a",
      registryPath: "/tmp/stp-registry.json",
      terminal,
    })

    expect(terminal.showCalls).toEqual([false])
  })

  test("terminates tracked active terminal after CLI succeeds", async () => {
    const store = new TerminalSessionStore<FakeTerminal>()
    const terminal = new FakeTerminal("terminal-a")
    store.trackOpenedSession({
      terminalId: "00000000-0000-0000-0000-000000000101",
      name: "STP: worktree-a 00000000",
      workspacePath: "/tmp/worktree-a",
      registryPath: "/tmp/stp-registry.json",
      terminal,
    })
    const runner = new RecordingRunner({ kind: "success", stdout: "terminated" })
    const messages = new RecordingMessages()
    let refreshCount = 0

    const result = await terminateCurrentTerminal({
      activeTerminal: terminal,
      binaryPath: "/opt/stp/bin/stp",
      messages,
      refresh: () => {
        refreshCount += 1
      },
      runner,
      store,
    })

    expect(result).toEqual({
      kind: "terminated",
      terminalId: "00000000-0000-0000-0000-000000000101",
    })
    expect(runner.calls).toEqual([
      {
        binaryPath: "/opt/stp/bin/stp",
        args: [
          "terminate",
          "--terminal-id",
          "00000000-0000-0000-0000-000000000101",
          "--yes",
          "--registry",
          "/tmp/stp-registry.json",
        ],
      },
    ])
    expect(terminal.disposeCount).toBe(1)
    expect(store.sessionForTerminal(terminal)).toBeUndefined()
    expect(refreshCount).toBe(1)
    expect(messages.information).toEqual([])
    expect(messages.errors).toEqual([])
  })

  test("does not dispose tracked terminal when CLI terminate fails", async () => {
    const store = new TerminalSessionStore<FakeTerminal>()
    const terminal = new FakeTerminal("terminal-a")
    store.trackOpenedSession({
      terminalId: "00000000-0000-0000-0000-000000000101",
      name: "STP: worktree-a 00000000",
      workspacePath: "/tmp/worktree-a",
      terminal,
    })
    const runner = new RecordingRunner({ kind: "failure", message: "tmux command failed" })
    const messages = new RecordingMessages()

    const result = await terminateCurrentTerminal({
      activeTerminal: terminal,
      binaryPath: "stp",
      messages,
      refresh: () => undefined,
      runner,
      store,
    })

    expect(result).toEqual({ kind: "failed", message: "tmux command failed" })
    expect(terminal.disposeCount).toBe(0)
    expect(store.sessionForTerminal(terminal)?.terminalId).toBe(
      "00000000-0000-0000-0000-000000000101",
    )
    expect(messages.errors).toEqual(["Failed to terminate STP terminal: tmux command failed"])
  })

  test("does not terminate untracked active terminal", async () => {
    const store = new TerminalSessionStore<FakeTerminal>()
    const terminal = new FakeTerminal("terminal-a")
    const runner = new RecordingRunner({ kind: "success", stdout: "terminated" })
    const messages = new RecordingMessages()

    const result = await terminateCurrentTerminal({
      activeTerminal: terminal,
      binaryPath: "stp",
      messages,
      refresh: () => undefined,
      runner,
      store,
    })

    expect(result).toEqual({ kind: "untracked" })
    expect(runner.calls).toEqual([])
    expect(terminal.disposeCount).toBe(0)
    expect(messages.information).toEqual(["The active terminal is not a tracked STP terminal"])
  })

  test("does not terminate when no terminal is active", async () => {
    const store = new TerminalSessionStore<FakeTerminal>()
    const runner = new RecordingRunner({ kind: "success", stdout: "terminated" })
    const messages = new RecordingMessages()

    const result = await terminateCurrentTerminal({
      activeTerminal: undefined,
      binaryPath: "stp",
      messages,
      refresh: () => undefined,
      runner,
      store,
    })

    expect(result).toEqual({ kind: "no-active-terminal" })
    expect(runner.calls).toEqual([])
    expect(messages.information).toEqual(["No active STP terminal to terminate"])
  })

  test("builds terminate CLI args from the terminal id contract", () => {
    expect(buildTerminateArgs("00000000-0000-0000-0000-000000000101", undefined)).toEqual([
      "terminate",
      "--terminal-id",
      "00000000-0000-0000-0000-000000000101",
      "--yes",
    ])
  })

  test("builds cleanup zombie CLI args from the registry path contract", () => {
    expect(buildCleanupZombiesArgs("/tmp/stp-registry.json")).toEqual([
      "registry",
      "cleanup-zombies",
      "--registry",
      "/tmp/stp-registry.json",
      "--yes",
    ])
  })

  test("runs cleanup zombie command through the configured binary", async () => {
    const runner = new RecordingRunner({ kind: "success", stdout: "removed zombie entries: 1" })

    const result = await cleanupZombieSessions({
      binaryPath: "/opt/stp/bin/stp",
      registryPath: "/tmp/stp-registry.json",
      runner,
    })

    expect(result).toEqual({ kind: "cleaned", stdout: "removed zombie entries: 1" })
    expect(runner.calls).toEqual([
      {
        binaryPath: "/opt/stp/bin/stp",
        args: [
          "registry",
          "cleanup-zombies",
          "--registry",
          "/tmp/stp-registry.json",
          "--yes",
        ],
      },
    ])
  })
})

class FakeTerminal {
  readonly showCalls: boolean[] = []
  disposeCount = 0

  constructor(readonly name: string) {}

  show(preserveFocus: boolean): void {
    this.showCalls.push(preserveFocus)
  }

  dispose(): void {
    this.disposeCount += 1
  }
}

class RecordingRunner implements CommandRunner {
  readonly calls: { readonly binaryPath: string; readonly args: readonly string[] }[] = []

  constructor(private readonly result: Awaited<ReturnType<CommandRunner["run"]>>) {}

  run(
    binaryPath: string,
    args: readonly string[],
  ): Promise<Awaited<ReturnType<CommandRunner["run"]>>> {
    this.calls.push({ binaryPath, args })
    return Promise.resolve(this.result)
  }
}

class RecordingMessages implements MessageSink {
  readonly information: string[] = []
  readonly errors: string[] = []

  showInformationMessage(message: string): void {
    this.information.push(message)
  }

  showErrorMessage(message: string): void {
    this.errors.push(message)
  }
}

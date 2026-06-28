import { describe, expect, test } from "bun:test"

import { terminateClosedTerminal, type CommandRunner } from "../src/terminalCommands"

describe("terminal close commands", () => {
  test("terminates a closed tracked terminal without disposing it again", async () => {
    const terminal = new FakeTerminal("terminal-a")
    const runner = new RecordingRunner({ kind: "success", stdout: "terminated" })

    const result = await terminateClosedTerminal({
      binaryPath: "/opt/stp/bin/stp",
      runner,
      session: {
        terminalId: "00000000-0000-0000-0000-000000000101",
        name: "STP: worktree-a 00000000",
        workspacePath: "/tmp/worktree-a",
        registryPath: "/tmp/stp-registry.json",
        terminal,
      },
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
    expect(terminal.disposeCount).toBe(0)
  })

  test("treats a closed regular tracked terminal as terminated without running the CLI", async () => {
    const terminal = new FakeTerminal("terminal-a")
    const runner = new RecordingRunner({ kind: "failure", message: "should not run" })

    const result = await terminateClosedTerminal({
      binaryPath: "/opt/stp/bin/stp",
      runner,
      session: {
        terminalId: "00000000-0000-0000-0000-000000000101",
        name: "STP: worktree-a 00000000",
        workspacePath: "/tmp/worktree-a",
        terminal,
      },
    })

    expect(result).toEqual({
      kind: "terminated",
      terminalId: "00000000-0000-0000-0000-000000000101",
    })
    expect(runner.calls).toEqual([])
    expect(terminal.disposeCount).toBe(0)
  })
})

class FakeTerminal {
  disposeCount = 0

  constructor(readonly name: string) {}
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

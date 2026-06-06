import { execFile } from "node:child_process"

import type { CommandResult } from "./terminalCommands"

export function runStpCommand(binaryPath: string, args: readonly string[]): Promise<CommandResult> {
  return new Promise((resolve) => {
    execFile(binaryPath, [...args], { timeout: 30_000 }, (error, stdout, stderr) => {
      if (error !== null) {
        const trimmedStderr = stderr.trim()
        resolve({
          kind: "failure",
          message: trimmedStderr.length > 0 ? trimmedStderr : error.message,
        })
        return
      }
      resolve({ kind: "success", stdout })
    })
  })
}

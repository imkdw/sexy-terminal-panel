import { randomUUID } from "node:crypto"
import * as vscode from "vscode"

import {
  NoWorkspaceError,
  buildTerminalArgs,
  selectBinaryPath,
  selectWorkspacePath,
} from "./terminalProfile"

const PROFILE_ID = "stp.tmuxTerminal"
const WINDOW_ID_KEY = "stp.windowId"

export function activate(context: vscode.ExtensionContext): void {
  const provider: vscode.TerminalProfileProvider = {
    async provideTerminalProfile() {
      try {
        const windowId = await windowIdForContext(context)
        const workspacePath = selectWorkspacePath(
          (vscode.workspace.workspaceFolders ?? []).map((folder) => ({
            path: folder.uri.fsPath,
          })),
        )
        const binaryPath = selectBinaryPath(
          vscode.workspace.getConfiguration("stp").inspect<string>("binaryPath"),
        )
        const args = buildTerminalArgs({
          binaryPath,
          workspacePath,
          windowId,
          terminalId: randomUUID(),
        })
        const shellPath = args[0]
        if (shellPath === undefined) {
          throw new NoWorkspaceError()
        }
        return new vscode.TerminalProfile({
          name: "STP: tmux",
          shellPath,
          shellArgs: args.slice(1),
        })
      } catch (error) {
        if (error instanceof NoWorkspaceError) {
          await vscode.window.showErrorMessage(error.message)
        }
        throw error
      }
    },
  }
  context.subscriptions.push(vscode.window.registerTerminalProfileProvider(PROFILE_ID, provider))
}

export function deactivate(): void {}

async function windowIdForContext(context: vscode.ExtensionContext): Promise<string> {
  const existing = context.workspaceState.get<string>(WINDOW_ID_KEY)
  if (existing !== undefined) {
    return existing
  }
  const next = randomUUID()
  await context.workspaceState.update(WINDOW_ID_KEY, next)
  return next
}

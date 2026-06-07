import { randomUUID } from "node:crypto"

import * as vscode from "vscode"

import { currentBinaryPath, currentRegistryPath, currentTmuxSocket } from "./extensionConfig"
import {
  NoWorkspaceError,
  buildTerminalArgs,
  selectWorkspacePath,
} from "./terminalProfile"
import { type PendingTerminalSession, TerminalSessionStore } from "./terminalSessions"

const WINDOW_ID_KEY = "stp.windowId"

export type CreatedStpTerminalProfile = Readonly<{
  pending: PendingTerminalSession
  profile: vscode.TerminalProfile
}>

export async function createStpTerminalProfile(
  context: vscode.ExtensionContext,
  sessions: TerminalSessionStore<vscode.Terminal>,
  existingSession?: PendingTerminalSession,
): Promise<CreatedStpTerminalProfile> {
  try {
    const windowId = await windowIdForContext(context)
    const workspacePath =
      existingSession?.workspacePath ??
      selectWorkspacePath(
        (vscode.workspace.workspaceFolders ?? []).map((folder) => ({
          path: folder.uri.fsPath,
        })),
      )
    const binaryPath = existingSession?.binaryPath ?? currentBinaryPath()
    const tmuxSocket = existingSession?.tmuxSocket ?? currentTmuxSocket()
    const registryPath = existingSession?.registryPath ?? currentRegistryPath()
    const terminalId = existingSession?.terminalId ?? randomUUID()
    const session = sessions.createPending({
      binaryPath,
      registryPath,
      terminalId,
      tmuxSocket,
      workspacePath,
    })
    const args = buildTerminalArgs({
      binaryPath,
      workspacePath,
      windowId,
      terminalId,
      tmuxSocket,
      registryPath,
    })
    const shellPath = args[0]
    if (shellPath === undefined) {
      throw new NoWorkspaceError()
    }
    return {
      pending: session,
      profile: new vscode.TerminalProfile({
        name: session.name,
        shellPath,
        shellArgs: args.slice(1),
      }),
    }
  } catch (error) {
    if (error instanceof NoWorkspaceError) {
      await vscode.window.showErrorMessage(error.message)
    }
    throw error
  }
}

async function windowIdForContext(context: vscode.ExtensionContext): Promise<string> {
  const existing = context.workspaceState.get<string>(WINDOW_ID_KEY)
  if (existing !== undefined) {
    return existing
  }
  const next = randomUUID()
  await context.workspaceState.update(WINDOW_ID_KEY, next)
  return next
}

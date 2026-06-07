import { randomUUID } from "node:crypto"

import * as vscode from "vscode"

import {
  NoWorkspaceError,
  buildRegularTerminalOptions,
  selectWorkspacePath,
} from "./terminalProfile"
import { type PendingTerminalSession, TerminalSessionStore } from "./terminalSessions"

export type CreatedStpTerminalProfile = Readonly<{
  pending: PendingTerminalSession
  profile: vscode.TerminalProfile
}>

export async function createStpTerminalProfile(
  sessions: TerminalSessionStore<vscode.Terminal>,
  existingSession?: PendingTerminalSession,
): Promise<CreatedStpTerminalProfile> {
  try {
    const workspacePath =
      existingSession?.workspacePath ??
      selectWorkspacePath(
        (vscode.workspace.workspaceFolders ?? []).map((folder) => ({
          path: folder.uri.fsPath,
        })),
      )
    const terminalId = existingSession?.terminalId ?? randomUUID()
    const session = sessions.createPending({
      terminalId,
      workspacePath,
    })
    const options = buildRegularTerminalOptions({
      name: session.name,
      workspacePath,
    })
    return {
      pending: session,
      profile: new vscode.TerminalProfile(options),
    }
  } catch (error) {
    if (error instanceof NoWorkspaceError) {
      await vscode.window.showErrorMessage(error.message)
    }
    throw error
  }
}

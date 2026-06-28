import { randomUUID } from "node:crypto"

import * as vscode from "vscode"

import {
  NoWorkspaceError,
  buildStpTerminalOptions,
  selectWorkspacePath,
} from "./terminalProfile"
import { type PendingTerminalSession, TerminalSessionStore } from "./terminalSessions"

export type StpTerminalProfileConfiguration = Readonly<{
  binaryPath: string
  registryPath: string
  tmuxSocket: string
}>

export type CreatedStpTerminalProfile = Readonly<{
  pending: PendingTerminalSession
  profile: vscode.TerminalProfile
}>

export async function createStpTerminalProfile(
  sessions: TerminalSessionStore<vscode.Terminal>,
  configuration: StpTerminalProfileConfiguration,
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
    const session = sessions.createPending(
      existingSession === undefined
        ? {
            terminalId,
            workspacePath,
            binaryPath: configuration.binaryPath,
            registryPath: configuration.registryPath,
            tmuxSocket: configuration.tmuxSocket,
          }
        : {
            ...existingSession,
            binaryPath: existingSession.binaryPath ?? configuration.binaryPath,
            registryPath: existingSession.registryPath ?? configuration.registryPath,
            tmuxSocket: existingSession.tmuxSocket ?? configuration.tmuxSocket,
          },
    )
    const options = buildStpTerminalOptions({
      name: session.name,
      workspacePath,
      binaryPath: session.binaryPath ?? configuration.binaryPath,
      registryPath: session.registryPath ?? configuration.registryPath,
      tmuxSocket: session.tmuxSocket ?? configuration.tmuxSocket,
      windowId: randomUUID(),
      terminalId,
    })
    return {
      pending: session,
      profile: new vscode.TerminalProfile({
        ...options,
        shellArgs: [...options.shellArgs],
      }),
    }
  } catch (error) {
    if (error instanceof NoWorkspaceError) {
      await vscode.window.showErrorMessage(error.message)
    }
    throw error
  }
}

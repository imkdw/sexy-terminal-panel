import * as vscode from "vscode"

import {
  cleanupZombieSessions,
  detachClosedTerminal,
  showTrackedTerminal,
  terminateCurrentTerminal,
} from "./terminalCommands"
import { currentBinaryPath, currentRegistryPath } from "./extensionConfig"
import { loadLiveRegistrySessions } from "./stpRegistry"
import { runStpCommand } from "./stpCommandRunner"
import { createStpTerminalProfile } from "./stpTerminalProfile"
import { TerminalSessionStore, type TerminalSession } from "./terminalSessions"
import { StpTerminalTreeProvider, type StpTerminalTreeItem } from "./terminalTree"

const PROFILE_ID = "stp.tmuxTerminal"
const SESSIONS_VIEW_ID = "stp.terminals"
const NEW_TERMINAL_COMMAND = "stp.newTerminal"
const SHOW_TERMINAL_COMMAND = "stp.showTerminal"
const TERMINATE_CURRENT_TERMINAL_COMMAND = "stp.terminateCurrentTerminal"

export function activate(context: vscode.ExtensionContext): void {
  const sessions = new TerminalSessionStore<vscode.Terminal>()
  const treeProvider = new StpTerminalTreeProvider(sessions, () =>
    loadLiveRegistrySessions(currentRegistryPath()),
  )
  const provider: vscode.TerminalProfileProvider = {
    async provideTerminalProfile() {
      const { profile } = await createStpTerminalProfile(context, sessions)
      return profile
    },
  }
  context.subscriptions.push(
    vscode.window.registerTerminalProfileProvider(PROFILE_ID, provider),
    vscode.window.registerTreeDataProvider(SESSIONS_VIEW_ID, treeProvider),
    vscode.commands.registerCommand(NEW_TERMINAL_COMMAND, async () => {
      const { pending, profile } = await createStpTerminalProfile(context, sessions)
      const terminal = vscode.window.createTerminal(profile.options)
      const session = sessions.attachOpenedTerminal({
        initialName: pending.name,
        name: pending.name,
        terminal,
      })
      if (session !== undefined) {
        treeProvider.refresh()
      }
      terminal.show(false)
      return terminal
    }),
    vscode.commands.registerCommand(
      SHOW_TERMINAL_COMMAND,
      (item: StpTerminalTreeItem<vscode.Terminal> | undefined) => {
        return showStpTerminalTreeItem(context, sessions, treeProvider, item)
      },
    ),
    vscode.commands.registerCommand(TERMINATE_CURRENT_TERMINAL_COMMAND, () => {
      return terminateCurrentStpTerminal(sessions, treeProvider)
    }),
    vscode.window.onDidOpenTerminal((terminal) => {
      const initialName = terminalCreationName(terminal)
      const session = sessions.attachOpenedTerminal(
        initialName === undefined
          ? { name: terminal.name, terminal }
          : { initialName, name: terminal.name, terminal },
      )
      if (session !== undefined) {
        treeProvider.refresh()
      }
    }),
    vscode.window.onDidCloseTerminal((terminal) => {
      const session = sessions.removeTerminal(terminal)
      if (session !== undefined) {
        void detachClosedStpTerminal(session, treeProvider)
        treeProvider.refresh()
      }
    }),
    treeProvider,
  )
  void cleanupZombieRegistry(treeProvider)
}

export function deactivate(): void {}

type StpTerminalSession = TerminalSession<vscode.Terminal>

function terminateCurrentStpTerminal(
  sessions: TerminalSessionStore<vscode.Terminal>,
  treeProvider: StpTerminalTreeProvider<vscode.Terminal>,
): Promise<unknown> {
  return terminateCurrentTerminal({
    activeTerminal: vscode.window.activeTerminal,
    binaryPath: currentBinaryPath(),
    messages: {
      showInformationMessage(message) {
        void vscode.window.showInformationMessage(message)
      },
      showErrorMessage(message) {
        void vscode.window.showErrorMessage(message)
      },
    },
    refresh() {
      treeProvider.refresh()
    },
    runner: { run: runStpCommand },
    store: sessions,
  })
}

async function showStpTerminalTreeItem(
  context: vscode.ExtensionContext,
  sessions: TerminalSessionStore<vscode.Terminal>,
  treeProvider: StpTerminalTreeProvider<vscode.Terminal>,
  item: StpTerminalTreeItem<vscode.Terminal> | undefined,
): Promise<void> {
  if (item === undefined) {
    return
  }
  if (item.kind === "opened") {
    showTrackedTerminal(item.session)
    return
  }
  const openedSession = sessions.sessionForId(item.session.terminalId)
  if (openedSession !== undefined) {
    showTrackedTerminal(openedSession)
    return
  }
  const { pending, profile } = await createStpTerminalProfile(context, sessions, {
    ...item.session,
    binaryPath: currentBinaryPath(),
    registryPath: currentRegistryPath(),
  })
  const terminal = vscode.window.createTerminal(profile.options)
  const session = sessions.attachOpenedTerminal({
    initialName: pending.name,
    name: pending.name,
    terminal,
  })
  if (session !== undefined) {
    treeProvider.refresh()
  }
  terminal.show(false)
}

async function detachClosedStpTerminal(
  session: StpTerminalSession,
  treeProvider: StpTerminalTreeProvider<vscode.Terminal>,
): Promise<void> {
  const result = await detachClosedTerminal({
    binaryPath: currentBinaryPath(),
    runner: { run: runStpCommand },
    session,
  })
  if (result.kind === "failed") {
    await vscode.window.showErrorMessage(`Failed to detach STP terminal: ${result.message}`)
  }
  treeProvider.refresh()
}

async function cleanupZombieRegistry(
  treeProvider: StpTerminalTreeProvider<vscode.Terminal>,
): Promise<void> {
  const result = await cleanupZombieSessions({
    binaryPath: currentBinaryPath(),
    registryPath: currentRegistryPath(),
    runner: { run: runStpCommand },
  })
  if (result.kind === "failed") {
    await vscode.window.showErrorMessage(`Failed to cleanup zombie STP sessions: ${result.message}`)
  }
  treeProvider.refresh()
}

function terminalCreationName(terminal: vscode.Terminal): string | undefined {
  const { creationOptions } = terminal
  return "name" in creationOptions && typeof creationOptions.name === "string"
    ? creationOptions.name
    : undefined
}

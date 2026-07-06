import * as vscode from "vscode"

import {
  cleanupZombieSessions,
  showTrackedTerminal,
  terminateClosedTerminal,
  terminateCurrentTerminal,
  terminateTrackedTerminals,
} from "./terminalCommands"
import { currentBinaryPath, currentRegistryPath, currentTmuxSocket } from "./extensionConfig"
import { loadLiveRegistrySessions } from "./stpRegistry"
import { runStpCommand } from "./stpCommandRunner"
import {
  createStpTerminalProfile,
  type StpTerminalProfileConfiguration,
} from "./stpTerminalProfile"
import { TerminalSessionStore, type TerminalSession } from "./terminalSessions"
import { StpTerminalTreeProvider, type StpTerminalTreeItem } from "./terminalTree"

const PROFILE_ID = "stp.terminal"
const SESSIONS_VIEW_ID = "stp.terminals"
const NEW_TERMINAL_COMMAND = "stp.newTerminal"
const SHOW_TERMINAL_COMMAND = "stp.showTerminal"
const TERMINATE_CURRENT_TERMINAL_COMMAND = "stp.terminateCurrentTerminal"

type ActiveExtension = Readonly<{
  sessions: TerminalSessionStore<vscode.Terminal>
}>

let activeExtension: ActiveExtension | undefined

export function activate(context: vscode.ExtensionContext): void {
  const sessions = new TerminalSessionStore<vscode.Terminal>()
  const treeProvider = new StpTerminalTreeProvider(sessions, () =>
    loadLiveRegistrySessions(currentRegistryPath()),
  )
  activeExtension = { sessions }
  const provider: vscode.TerminalProfileProvider = {
    async provideTerminalProfile() {
      const { profile } = await createStpTerminalProfile(sessions, currentTerminalProfileConfig())
      return profile
    },
  }
  context.subscriptions.push(
    vscode.window.registerTerminalProfileProvider(PROFILE_ID, provider),
    vscode.window.registerTreeDataProvider(SESSIONS_VIEW_ID, treeProvider),
    vscode.commands.registerCommand(NEW_TERMINAL_COMMAND, async () => {
      const { pending, profile } = await createStpTerminalProfile(
        sessions,
        currentTerminalProfileConfig(),
      )
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
        return showStpTerminalTreeItem(sessions, treeProvider, item)
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
        void terminateClosedStpTerminal(session, treeProvider)
        treeProvider.refresh()
      }
    }),
    treeProvider,
  )
  void cleanupZombieRegistry(treeProvider)
}

export async function deactivate(): Promise<void> {
  const extension = activeExtension
  activeExtension = undefined
  if (extension === undefined) {
    return
  }

  const result = await terminateTrackedTerminals({
    binaryPath: currentBinaryPath(),
    runner: { run: runStpCommand },
    store: extension.sessions,
  })
  if (result.failures.length > 0) {
    await vscode.window.showErrorMessage(
      `Failed to terminate STP terminal sessions: ${result.failures
        .map((failure) => `${failure.terminalId}: ${failure.message}`)
        .join("; ")}`,
    )
  }
}

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
  const { pending, profile } = await createStpTerminalProfile(
    sessions,
    currentTerminalProfileConfig(),
    item.session,
  )
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

async function terminateClosedStpTerminal(
  session: StpTerminalSession,
  treeProvider: StpTerminalTreeProvider<vscode.Terminal>,
): Promise<void> {
  const result = await terminateClosedTerminal({
    binaryPath: currentBinaryPath(),
    runner: { run: runStpCommand },
    session,
  })
  if (result.kind === "failed") {
    await vscode.window.showErrorMessage(`Failed to terminate STP terminal: ${result.message}`)
  }
  treeProvider.refresh()
}

function currentTerminalProfileConfig(): StpTerminalProfileConfiguration {
  return {
    binaryPath: currentBinaryPath(),
    registryPath: currentRegistryPath(),
    tmuxSocket: currentTmuxSocket(),
  }
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

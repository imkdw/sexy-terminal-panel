import type { TerminalSession, TerminalSessionStore } from "./terminalSessions"

export type TerminalController = Readonly<{
  name: string
  show(preserveFocus: boolean): void
  dispose(): void
}>

export type CommandResult =
  | Readonly<{ kind: "success"; stdout: string }>
  | Readonly<{ kind: "failure"; message: string }>

export type CommandRunner = Readonly<{
  run(binaryPath: string, args: readonly string[]): Promise<CommandResult>
}>

export type MessageSink = Readonly<{
  showInformationMessage(message: string): void
  showErrorMessage(message: string): void
}>

export type TerminationResult =
  | Readonly<{ kind: "terminated"; terminalId: string }>
  | Readonly<{ kind: "failed"; message: string }>
  | Readonly<{ kind: "no-active-terminal" }>
  | Readonly<{ kind: "untracked" }>

export type ClosedTerminalTerminationResult =
  | Readonly<{ kind: "terminated"; terminalId: string }>
  | Readonly<{ kind: "failed"; message: string }>

export type CleanupResult =
  | Readonly<{ kind: "cleaned"; stdout: string }>
  | Readonly<{ kind: "failed"; message: string }>

export type TerminateCurrentTerminalInput<TTerminal extends TerminalController> = Readonly<{
  activeTerminal: TTerminal | undefined
  binaryPath: string
  messages: MessageSink
  refresh(): void
  runner: CommandRunner
  store: TerminalSessionStore<TTerminal>
}>

export function showTrackedTerminal<TTerminal extends TerminalController>(
  session: TerminalSession<TTerminal>,
): void {
  session.terminal.show(false)
}

export async function terminateCurrentTerminal<TTerminal extends TerminalController>(
  input: TerminateCurrentTerminalInput<TTerminal>,
): Promise<TerminationResult> {
  const activeTerminal = input.activeTerminal
  if (activeTerminal === undefined) {
    input.messages.showInformationMessage("No active STP terminal to terminate")
    return { kind: "no-active-terminal" }
  }

  const session = input.store.sessionForTerminal(activeTerminal)
  if (session === undefined) {
    input.messages.showInformationMessage("The active terminal is not a tracked STP terminal")
    return { kind: "untracked" }
  }

  if (!usesRegistryCommand(session)) {
    input.store.removeTerminal(activeTerminal)
    activeTerminal.dispose()
    input.refresh()
    return { kind: "terminated", terminalId: session.terminalId }
  }

  const binaryPath = session.binaryPath ?? input.binaryPath
  const result = await input.runner.run(
    binaryPath,
    buildTerminateArgs(session.terminalId, session.registryPath),
  )
  if (result.kind === "failure") {
    input.messages.showErrorMessage(`Failed to terminate STP terminal: ${result.message}`)
    return { kind: "failed", message: result.message }
  }

  input.store.removeTerminal(activeTerminal)
  activeTerminal.dispose()
  input.refresh()
  return { kind: "terminated", terminalId: session.terminalId }
}

export async function terminateClosedTerminal<TTerminal>(input: {
  readonly binaryPath: string
  readonly runner: CommandRunner
  readonly session: TerminalSession<TTerminal>
}): Promise<ClosedTerminalTerminationResult> {
  if (!usesRegistryCommand(input.session)) {
    return { kind: "terminated", terminalId: input.session.terminalId }
  }

  const binaryPath = input.session.binaryPath ?? input.binaryPath
  const result = await input.runner.run(
    binaryPath,
    buildTerminateArgs(input.session.terminalId, input.session.registryPath),
  )
  if (result.kind === "failure") {
    return { kind: "failed", message: result.message }
  }
  return { kind: "terminated", terminalId: input.session.terminalId }
}

export async function cleanupZombieSessions(input: {
  readonly binaryPath: string
  readonly registryPath: string
  readonly runner: CommandRunner
}): Promise<CleanupResult> {
  const result = await input.runner.run(
    input.binaryPath,
    buildCleanupZombiesArgs(input.registryPath),
  )
  if (result.kind === "failure") {
    return { kind: "failed", message: result.message }
  }
  return { kind: "cleaned", stdout: result.stdout }
}

export function buildTerminateArgs(
  terminalId: string,
  registryPath: string | undefined,
): readonly string[] {
  const args = ["terminate", "--terminal-id", terminalId, "--yes"]
  if (registryPath !== undefined && registryPath.length > 0) {
    return [...args, "--registry", registryPath]
  }
  return args
}

export function buildCleanupZombiesArgs(registryPath: string): readonly string[] {
  return ["registry", "cleanup-zombies", "--registry", registryPath, "--yes"]
}

function usesRegistryCommand<TTerminal>(session: TerminalSession<TTerminal>): boolean {
  return session.registryPath !== undefined && session.registryPath.length > 0
}

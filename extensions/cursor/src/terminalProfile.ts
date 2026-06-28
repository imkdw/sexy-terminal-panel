export type StpTerminalOptions = Readonly<{
  name: string
  cwd: string
  shellPath: string
  shellArgs: readonly string[]
}>

export type StpTerminalInput = Readonly<{
  name: string
  workspacePath: string
  binaryPath: string
  registryPath: string
  tmuxSocket: string
  windowId: string
  terminalId: string
}>

export type WorkspaceCandidate = Readonly<{
  path: string
}>

export type BinaryPathConfiguration = Readonly<{
  defaultValue?: string
  globalValue?: string
}>

export type TmuxSocketConfiguration = Readonly<{
  defaultValue?: string
  globalValue?: string
}>

export class NoWorkspaceError extends Error {
  constructor() {
    super("No workspace folder is open")
    this.name = "NoWorkspaceError"
  }
}

export function buildStpTerminalOptions(input: StpTerminalInput): StpTerminalOptions {
  return {
    name: input.name,
    cwd: input.workspacePath,
    shellPath: input.binaryPath,
    shellArgs: [
      "terminal",
      "--workspace",
      input.workspacePath,
      "--window-id",
      input.windowId,
      "--terminal-id",
      input.terminalId,
      "--registry",
      input.registryPath,
      "--socket",
      input.tmuxSocket,
    ],
  }
}

export function selectWorkspacePath(workspaces: readonly WorkspaceCandidate[]): string {
  const first = workspaces[0]
  if (first === undefined) {
    throw new NoWorkspaceError()
  }
  return first.path
}

export function selectBinaryPath(configuration: BinaryPathConfiguration | undefined): string {
  if (configuration?.globalValue !== undefined && configuration.globalValue.length > 0) {
    return configuration.globalValue
  }
  if (configuration?.defaultValue !== undefined && configuration.defaultValue.length > 0) {
    return configuration.defaultValue
  }
  return "stp"
}

export function selectTmuxSocket(configuration: TmuxSocketConfiguration | undefined): string {
  if (configuration?.globalValue !== undefined && configuration.globalValue.length > 0) {
    return configuration.globalValue
  }
  if (configuration?.defaultValue !== undefined && configuration.defaultValue.length > 0) {
    return configuration.defaultValue
  }
  return "stp-managed"
}

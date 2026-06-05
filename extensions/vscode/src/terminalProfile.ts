export type TerminalProfileInput = Readonly<{
  binaryPath: string
  workspacePath: string
  windowId: string
  terminalId: string
  shellPath?: string
}>

export type WorkspaceCandidate = Readonly<{
  path: string
}>

export type BinaryPathConfiguration = Readonly<{
  defaultValue?: string
  globalValue?: string
}>

export class NoWorkspaceError extends Error {
  constructor() {
    super("No workspace folder is open")
    this.name = "NoWorkspaceError"
  }
}

export function buildTerminalArgs(input: TerminalProfileInput): readonly string[] {
  const args = [
    input.binaryPath,
    "terminal",
    "--workspace",
    input.workspacePath,
    "--window-id",
    input.windowId,
    "--terminal-id",
    input.terminalId,
  ]
  if (input.shellPath !== undefined && input.shellPath.length > 0) {
    return [...args, "--shell", input.shellPath]
  }
  return args
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

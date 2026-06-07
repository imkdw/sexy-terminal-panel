export type RegularTerminalOptions = Readonly<{
  name: string
  cwd: string
}>

export type RegularTerminalInput = Readonly<{
  name: string
  workspacePath: string
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

export function buildRegularTerminalOptions(
  input: RegularTerminalInput,
): RegularTerminalOptions {
  return {
    name: input.name,
    cwd: input.workspacePath,
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

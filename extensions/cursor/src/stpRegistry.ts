import { existsSync, readFileSync } from "node:fs"
import { homedir } from "node:os"
import { join } from "node:path"

import { buildTerminalSessionName, type PendingTerminalSession } from "./terminalSessions"

export type RegistryPathConfiguration = Readonly<{
  defaultValue?: string
  globalValue?: string
}>

export type RegistryTerminalStatus = "starting" | "live" | "detached" | "stale" | "exited"

export type RegistryTerminalSession = Readonly<
  PendingTerminalSession & {
    status: RegistryTerminalStatus
  }
>

export function loadLiveRegistrySessions(registryPath: string): readonly RegistryTerminalSession[] {
  if (!existsSync(registryPath)) {
    return []
  }
  const parsed: unknown = JSON.parse(readFileSync(registryPath, "utf8"))
  if (typeof parsed !== "object" || parsed === null) {
    return []
  }
  const terminals: unknown = Reflect.get(parsed, "terminals")
  if (!Array.isArray(terminals)) {
    return []
  }
  return terminals.flatMap((terminal) => {
    const parsedTerminal = parseRegistryTerminal(terminal)
    if (parsedTerminal === undefined || parsedTerminal.status !== "live") {
      return []
    }
    return [parsedTerminal]
  })
}

export function selectRegistryPath(
  configuration: RegistryPathConfiguration | undefined,
  env: NodeJS.ProcessEnv = process.env,
): string {
  if (configuration?.globalValue !== undefined && configuration.globalValue.length > 0) {
    return configuration.globalValue
  }
  if (configuration?.defaultValue !== undefined && configuration.defaultValue.length > 0) {
    return configuration.defaultValue
  }
  return defaultRegistryPath(env)
}

function defaultRegistryPath(env: NodeJS.ProcessEnv): string {
  const stateHome = env["XDG_STATE_HOME"]
  if (stateHome !== undefined && stateHome.length > 0) {
    return join(stateHome, "sexy-terminal-panel", "registry.json")
  }
  const home = env["HOME"]
  if (home !== undefined && home.length > 0) {
    return join(home, ".local", "state", "sexy-terminal-panel", "registry.json")
  }
  return join(homedir(), ".local", "state", "sexy-terminal-panel", "registry.json")
}

function parseRegistryTerminal(value: unknown): RegistryTerminalSession | undefined {
  if (typeof value !== "object" || value === null) {
    return undefined
  }
  const terminalId = readString(value, "terminal_id")
  const workspacePath = readString(value, "workspace_path")
  const tmuxSocket = readString(value, "tmux_socket")
  const tmuxSession = readString(value, "tmux_session")
  const status = parseStatus(readString(value, "status") ?? "live")
  if (
    terminalId === undefined ||
    workspacePath === undefined ||
    tmuxSocket === undefined ||
    tmuxSession === undefined ||
    status === undefined
  ) {
    return undefined
  }
  return {
    name: buildTerminalSessionName({ terminalId, workspacePath }),
    terminalId,
    workspacePath,
    tmuxSocket,
    tmuxSession,
    status,
  }
}

function readString(record: object, key: string): string | undefined {
  const value: unknown = Reflect.get(record, key)
  return typeof value === "string" ? value : undefined
}

function parseStatus(value: string): RegistryTerminalStatus | undefined {
  switch (value) {
    case "starting":
    case "live":
    case "detached":
    case "stale":
    case "exited":
      return value
  }
  return undefined
}

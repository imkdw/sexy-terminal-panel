import { existsSync, readFileSync } from "node:fs"
import { homedir } from "node:os"
import { join } from "node:path"

import { buildTerminalSessionName, type PendingTerminalSession } from "./terminalSessions"

export type RegistryPathConfiguration = Readonly<{
  defaultValue?: string
  globalValue?: string
}>

export type RegistryTerminalStatus = "starting" | "live" | "detached" | "stale" | "exited"

export type RegistryTerminalBackend =
  | Readonly<{
      kind: "pty"
      endpoint: Readonly<{
        socketPath: string
      }>
    }>
  | Readonly<{
      kind: "legacy-tmux"
      socket: string
      session: string
      window?: string
    }>

export type RegistryTerminalSession = Readonly<
  PendingTerminalSession & {
    backend?: RegistryTerminalBackend
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
  const backend = parseBackend(value)
  const status = parseStatus(readString(value, "status") ?? "live")
  if (terminalId === undefined || workspacePath === undefined || backend === undefined || status === undefined) {
    return undefined
  }
  const session = {
    name: buildTerminalSessionName({ terminalId, workspacePath }),
    terminalId,
    workspacePath,
    backend,
    status,
  }
  switch (backend.kind) {
    case "legacy-tmux":
      return {
        ...session,
        tmuxSocket: backend.socket,
        tmuxSession: backend.session,
      }
    case "pty":
      return session
  }
}

function readString(record: object, key: string): string | undefined {
  const value: unknown = Reflect.get(record, key)
  return typeof value === "string" ? value : undefined
}

function parseBackend(record: object): RegistryTerminalBackend | undefined {
  const backend: unknown = Reflect.get(record, "backend")
  if (typeof backend === "object" && backend !== null) {
    return parseStructuredBackend(backend)
  }
  const tmuxSocket = readString(record, "tmux_socket")
  const tmuxSession = readString(record, "tmux_session")
  if (tmuxSocket === undefined || tmuxSession === undefined) {
    return undefined
  }
  return legacyTmuxBackend(tmuxSocket, tmuxSession, readString(record, "tmux_window"))
}

function parseStructuredBackend(record: object): RegistryTerminalBackend | undefined {
  const kind = readString(record, "kind")
  switch (kind) {
    case "pty":
      return parsePtyBackend(record)
    case "legacy-tmux":
      return parseLegacyTmuxBackend(record)
    default:
      return undefined
  }
}

function parsePtyBackend(record: object): RegistryTerminalBackend | undefined {
  const endpoint: unknown = Reflect.get(record, "endpoint")
  if (typeof endpoint !== "object" || endpoint === null) {
    return undefined
  }
  const socketPath = readString(endpoint, "socket_path")
  if (socketPath === undefined) {
    return undefined
  }
  return {
    kind: "pty",
    endpoint: {
      socketPath,
    },
  }
}

function parseLegacyTmuxBackend(record: object): RegistryTerminalBackend | undefined {
  const socket = readString(record, "socket")
  const session = readString(record, "session")
  if (socket === undefined || session === undefined) {
    return undefined
  }
  return legacyTmuxBackend(socket, session, readString(record, "window"))
}

function legacyTmuxBackend(
  socket: string,
  session: string,
  window: string | undefined,
): RegistryTerminalBackend {
  if (window === undefined) {
    return {
      kind: "legacy-tmux",
      socket,
      session,
    }
  }
  return {
    kind: "legacy-tmux",
    socket,
    session,
    window,
  }
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

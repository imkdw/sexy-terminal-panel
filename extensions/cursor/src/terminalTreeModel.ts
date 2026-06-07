import type { RegistryTerminalSession } from "./stpRegistry"
import type { TerminalSession } from "./terminalSessions"

export type StpTerminalTreeItem<TTerminal> =
  | Readonly<{ kind: "opened"; session: TerminalSession<TTerminal> }>
  | Readonly<{ kind: "registry"; session: RegistryTerminalSession }>

export function mergeTerminalTreeItems<TTerminal>(
  openedSessions: readonly TerminalSession<TTerminal>[],
  registrySessions: readonly RegistryTerminalSession[],
): StpTerminalTreeItem<TTerminal>[] {
  const openedIds = new Set(openedSessions.map((session) => session.terminalId))
  const registryItems = registrySessions
    .filter((session) => !openedIds.has(session.terminalId))
    .map((session) => ({ kind: "registry", session }) satisfies StpTerminalTreeItem<TTerminal>)
  return [
    ...openedSessions.map(
      (session) => ({ kind: "opened", session }) satisfies StpTerminalTreeItem<TTerminal>,
    ),
    ...registryItems,
  ]
}

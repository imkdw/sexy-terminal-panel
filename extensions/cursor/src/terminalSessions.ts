import { basename } from "node:path"

const STP_TERMINAL_PREFIX = "STP:"
const TERMINAL_ID_LABEL_LENGTH = 8
const STP_TERMINAL_NAME_PATTERN = /^STP: .+ [0-9a-fA-F]{8}$/

export type TerminalSessionInput = Readonly<{
  terminalId: string
  workspacePath: string
  binaryPath?: string
  registryPath?: string
  tmuxSocket?: string
  tmuxSession?: string
}>

export type PendingTerminalSession = Readonly<
  TerminalSessionInput & {
    name: string
  }
>

export type TerminalSession<TTerminal> = Readonly<
  PendingTerminalSession & {
    terminal: TTerminal
  }
>

export type OpenedTerminalInput<TTerminal> = Readonly<{
  name: string
  initialName?: string
  terminal: TTerminal
}>

export class TerminalSessionStore<TTerminal> {
  private readonly pendingByName = new Map<string, PendingTerminalSession>()
  private readonly sessionsByTerminal = new Map<TTerminal, TerminalSession<TTerminal>>()
  private readonly sessionsById = new Map<string, TerminalSession<TTerminal>>()

  createPending(input: TerminalSessionInput): PendingTerminalSession {
    const session = {
      ...input,
      name: buildTerminalSessionName(input),
    }
    this.pendingByName.set(session.name, session)
    return session
  }

  attachOpenedTerminal(
    input: OpenedTerminalInput<TTerminal>,
  ): TerminalSession<TTerminal> | undefined {
    const pending =
      (input.initialName === undefined ? undefined : this.pendingByName.get(input.initialName)) ??
      this.pendingByName.get(input.name)
    if (pending === undefined) {
      return undefined
    }
    const session = {
      ...pending,
      terminal: input.terminal,
    }
    if (input.initialName !== undefined) {
      this.pendingByName.delete(input.initialName)
    }
    this.pendingByName.delete(input.name)
    this.sessionsByTerminal.set(input.terminal, session)
    this.sessionsById.set(session.terminalId, session)
    return session
  }

  removeTerminal(terminal: TTerminal): TerminalSession<TTerminal> | undefined {
    const session = this.sessionsByTerminal.get(terminal)
    if (session === undefined) {
      return undefined
    }
    this.sessionsByTerminal.delete(terminal)
    this.sessionsById.delete(session.terminalId)
    return session
  }

  drainSessions(): readonly TerminalSession<TTerminal>[] {
    const sessions = this.sessions()
    this.sessionsByTerminal.clear()
    this.sessionsById.clear()
    return sessions
  }

  trackOpenedSession(session: TerminalSession<TTerminal>): TerminalSession<TTerminal> {
    this.sessionsByTerminal.set(session.terminal, session)
    this.sessionsById.set(session.terminalId, session)
    return session
  }

  sessionForTerminal(terminal: TTerminal): TerminalSession<TTerminal> | undefined {
    return this.sessionsByTerminal.get(terminal)
  }

  sessionForId(terminalId: string): TerminalSession<TTerminal> | undefined {
    return this.sessionsById.get(terminalId)
  }

  sessions(): readonly TerminalSession<TTerminal>[] {
    return [...this.sessionsById.values()]
  }
}

export function buildTerminalSessionName(input: TerminalSessionInput): string {
  const workspaceName = basename(input.workspacePath) || input.workspacePath
  return `${STP_TERMINAL_PREFIX} ${workspaceName} ${shortTerminalId(input.terminalId)}`
}

export function shortTerminalId(terminalId: string): string {
  return terminalId.slice(0, TERMINAL_ID_LABEL_LENGTH)
}

export function isStpTerminalName(name: string): boolean {
  return STP_TERMINAL_NAME_PATTERN.test(name)
}

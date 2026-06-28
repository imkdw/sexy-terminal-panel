// src/extension.ts
import * as vscode4 from "vscode";

// src/terminalCommands.ts
function showTrackedTerminal(session) {
  session.terminal.show(false);
}
async function terminateCurrentTerminal(input) {
  const activeTerminal = input.activeTerminal;
  if (activeTerminal === undefined) {
    input.messages.showInformationMessage("No active STP terminal to terminate");
    return { kind: "no-active-terminal" };
  }
  const session = input.store.sessionForTerminal(activeTerminal);
  if (session === undefined) {
    input.messages.showInformationMessage("The active terminal is not a tracked STP terminal");
    return { kind: "untracked" };
  }
  if (!usesRegistryCommand(session)) {
    input.store.removeTerminal(activeTerminal);
    activeTerminal.dispose();
    input.refresh();
    return { kind: "terminated", terminalId: session.terminalId };
  }
  const binaryPath = session.binaryPath ?? input.binaryPath;
  const result = await input.runner.run(binaryPath, buildTerminateArgs(session.terminalId, session.registryPath));
  if (result.kind === "failure") {
    input.messages.showErrorMessage(`Failed to terminate STP terminal: ${result.message}`);
    return { kind: "failed", message: result.message };
  }
  input.store.removeTerminal(activeTerminal);
  activeTerminal.dispose();
  input.refresh();
  return { kind: "terminated", terminalId: session.terminalId };
}
async function terminateClosedTerminal(input) {
  if (!usesRegistryCommand(input.session)) {
    return { kind: "terminated", terminalId: input.session.terminalId };
  }
  const binaryPath = input.session.binaryPath ?? input.binaryPath;
  const result = await input.runner.run(binaryPath, buildTerminateArgs(input.session.terminalId, input.session.registryPath));
  if (result.kind === "failure") {
    return { kind: "failed", message: result.message };
  }
  return { kind: "terminated", terminalId: input.session.terminalId };
}
async function cleanupZombieSessions(input) {
  const result = await input.runner.run(input.binaryPath, buildCleanupZombiesArgs(input.registryPath));
  if (result.kind === "failure") {
    return { kind: "failed", message: result.message };
  }
  return { kind: "cleaned", stdout: result.stdout };
}
function buildTerminateArgs(terminalId, registryPath) {
  const args = ["terminate", "--terminal-id", terminalId, "--yes"];
  if (registryPath !== undefined && registryPath.length > 0) {
    return [...args, "--registry", registryPath];
  }
  return args;
}
function buildCleanupZombiesArgs(registryPath) {
  return ["registry", "cleanup-zombies", "--registry", registryPath, "--yes"];
}
function usesRegistryCommand(session) {
  return session.registryPath !== undefined && session.registryPath.length > 0;
}

// src/extensionConfig.ts
import * as vscode from "vscode";

// src/stpRegistry.ts
import { existsSync, readFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

// src/terminalSessions.ts
import { basename } from "node:path";
var STP_TERMINAL_PREFIX = "STP:";
var TERMINAL_ID_LABEL_LENGTH = 8;
class TerminalSessionStore {
  pendingByName = new Map;
  sessionsByTerminal = new Map;
  sessionsById = new Map;
  createPending(input) {
    const session = {
      ...input,
      name: buildTerminalSessionName(input)
    };
    this.pendingByName.set(session.name, session);
    return session;
  }
  attachOpenedTerminal(input) {
    const pending = (input.initialName === undefined ? undefined : this.pendingByName.get(input.initialName)) ?? this.pendingByName.get(input.name);
    if (pending === undefined) {
      return;
    }
    const session = {
      ...pending,
      terminal: input.terminal
    };
    if (input.initialName !== undefined) {
      this.pendingByName.delete(input.initialName);
    }
    this.pendingByName.delete(input.name);
    this.sessionsByTerminal.set(input.terminal, session);
    this.sessionsById.set(session.terminalId, session);
    return session;
  }
  removeTerminal(terminal) {
    const session = this.sessionsByTerminal.get(terminal);
    if (session === undefined) {
      return;
    }
    this.sessionsByTerminal.delete(terminal);
    this.sessionsById.delete(session.terminalId);
    return session;
  }
  trackOpenedSession(session) {
    this.sessionsByTerminal.set(session.terminal, session);
    this.sessionsById.set(session.terminalId, session);
    return session;
  }
  sessionForTerminal(terminal) {
    return this.sessionsByTerminal.get(terminal);
  }
  sessionForId(terminalId) {
    return this.sessionsById.get(terminalId);
  }
  sessions() {
    return [...this.sessionsById.values()];
  }
}
function buildTerminalSessionName(input) {
  const workspaceName = basename(input.workspacePath) || input.workspacePath;
  return `${STP_TERMINAL_PREFIX} ${workspaceName} ${shortTerminalId(input.terminalId)}`;
}
function shortTerminalId(terminalId) {
  return terminalId.slice(0, TERMINAL_ID_LABEL_LENGTH);
}

// src/stpRegistry.ts
function loadLiveRegistrySessions(registryPath) {
  if (!existsSync(registryPath)) {
    return [];
  }
  const parsed = JSON.parse(readFileSync(registryPath, "utf8"));
  if (typeof parsed !== "object" || parsed === null) {
    return [];
  }
  const terminals = Reflect.get(parsed, "terminals");
  if (!Array.isArray(terminals)) {
    return [];
  }
  return terminals.flatMap((terminal) => {
    const parsedTerminal = parseRegistryTerminal(terminal);
    if (parsedTerminal === undefined || parsedTerminal.status !== "live") {
      return [];
    }
    return [parsedTerminal];
  });
}
function selectRegistryPath(configuration, env = process.env) {
  if (configuration?.globalValue !== undefined && configuration.globalValue.length > 0) {
    return configuration.globalValue;
  }
  if (configuration?.defaultValue !== undefined && configuration.defaultValue.length > 0) {
    return configuration.defaultValue;
  }
  return defaultRegistryPath(env);
}
function defaultRegistryPath(env) {
  const stateHome = env["XDG_STATE_HOME"];
  if (stateHome !== undefined && stateHome.length > 0) {
    return join(stateHome, "sexy-terminal-panel", "registry.json");
  }
  const home = env["HOME"];
  if (home !== undefined && home.length > 0) {
    return join(home, ".local", "state", "sexy-terminal-panel", "registry.json");
  }
  return join(homedir(), ".local", "state", "sexy-terminal-panel", "registry.json");
}
function parseRegistryTerminal(value) {
  if (typeof value !== "object" || value === null) {
    return;
  }
  const terminalId = readString(value, "terminal_id");
  const workspacePath = readString(value, "workspace_path");
  const backend = parseBackend(value);
  const status = parseStatus(readString(value, "status") ?? "live");
  if (terminalId === undefined || workspacePath === undefined || backend === undefined || status === undefined) {
    return;
  }
  const session = {
    name: buildTerminalSessionName({ terminalId, workspacePath }),
    terminalId,
    workspacePath,
    backend,
    status
  };
  switch (backend.kind) {
    case "legacy-tmux":
      return {
        ...session,
        tmuxSocket: backend.socket,
        tmuxSession: backend.session
      };
    case "pty":
      return session;
  }
}
function readString(record, key) {
  const value = Reflect.get(record, key);
  return typeof value === "string" ? value : undefined;
}
function parseBackend(record) {
  const backend = Reflect.get(record, "backend");
  if (typeof backend === "object" && backend !== null) {
    return parseStructuredBackend(backend);
  }
  const tmuxSocket = readString(record, "tmux_socket");
  const tmuxSession = readString(record, "tmux_session");
  if (tmuxSocket === undefined || tmuxSession === undefined) {
    return;
  }
  return legacyTmuxBackend(tmuxSocket, tmuxSession, readString(record, "tmux_window"));
}
function parseStructuredBackend(record) {
  const kind = readString(record, "kind");
  switch (kind) {
    case "pty":
      return parsePtyBackend(record);
    case "legacy-tmux":
      return parseLegacyTmuxBackend(record);
    default:
      return;
  }
}
function parsePtyBackend(record) {
  const endpoint = Reflect.get(record, "endpoint");
  if (typeof endpoint !== "object" || endpoint === null) {
    return;
  }
  const socketPath = readString(endpoint, "socket_path");
  if (socketPath === undefined) {
    return;
  }
  return {
    kind: "pty",
    endpoint: {
      socketPath
    }
  };
}
function parseLegacyTmuxBackend(record) {
  const socket = readString(record, "socket");
  const session = readString(record, "session");
  if (socket === undefined || session === undefined) {
    return;
  }
  return legacyTmuxBackend(socket, session, readString(record, "window"));
}
function legacyTmuxBackend(socket, session, window) {
  if (window === undefined) {
    return {
      kind: "legacy-tmux",
      socket,
      session
    };
  }
  return {
    kind: "legacy-tmux",
    socket,
    session,
    window
  };
}
function parseStatus(value) {
  switch (value) {
    case "starting":
    case "live":
    case "detached":
    case "stale":
    case "exited":
      return value;
  }
  return;
}

// src/terminalProfile.ts
class NoWorkspaceError extends Error {
  constructor() {
    super("No workspace folder is open");
    this.name = "NoWorkspaceError";
  }
}
function buildStpTerminalOptions(input) {
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
      input.tmuxSocket
    ]
  };
}
function selectWorkspacePath(workspaces) {
  const first = workspaces[0];
  if (first === undefined) {
    throw new NoWorkspaceError;
  }
  return first.path;
}
function selectBinaryPath(configuration) {
  if (configuration?.globalValue !== undefined && configuration.globalValue.length > 0) {
    return configuration.globalValue;
  }
  if (configuration?.defaultValue !== undefined && configuration.defaultValue.length > 0) {
    return configuration.defaultValue;
  }
  return "stp";
}
function selectTmuxSocket(configuration) {
  if (configuration?.globalValue !== undefined && configuration.globalValue.length > 0) {
    return configuration.globalValue;
  }
  if (configuration?.defaultValue !== undefined && configuration.defaultValue.length > 0) {
    return configuration.defaultValue;
  }
  return "stp-managed";
}

// src/extensionConfig.ts
function currentBinaryPath() {
  return selectBinaryPath(vscode.workspace.getConfiguration("stp").inspect("binaryPath"));
}
function currentTmuxSocket() {
  return selectTmuxSocket(vscode.workspace.getConfiguration("stp").inspect("tmuxSocket"));
}
function currentRegistryPath() {
  return selectRegistryPath(vscode.workspace.getConfiguration("stp").inspect("registryPath"));
}

// src/stpCommandRunner.ts
import { execFile } from "node:child_process";
function runStpCommand(binaryPath, args) {
  return new Promise((resolve) => {
    execFile(binaryPath, [...args], { timeout: 30000 }, (error, stdout, stderr) => {
      if (error !== null) {
        const trimmedStderr = stderr.trim();
        resolve({
          kind: "failure",
          message: trimmedStderr.length > 0 ? trimmedStderr : error.message
        });
        return;
      }
      resolve({ kind: "success", stdout });
    });
  });
}

// src/stpTerminalProfile.ts
import { randomUUID } from "node:crypto";
import * as vscode2 from "vscode";
async function createStpTerminalProfile(sessions, configuration, existingSession) {
  try {
    const workspacePath = existingSession?.workspacePath ?? selectWorkspacePath((vscode2.workspace.workspaceFolders ?? []).map((folder) => ({
      path: folder.uri.fsPath
    })));
    const terminalId = existingSession?.terminalId ?? randomUUID();
    const session = sessions.createPending(existingSession === undefined ? {
      terminalId,
      workspacePath,
      binaryPath: configuration.binaryPath,
      registryPath: configuration.registryPath,
      tmuxSocket: configuration.tmuxSocket
    } : {
      ...existingSession,
      binaryPath: existingSession.binaryPath ?? configuration.binaryPath,
      registryPath: existingSession.registryPath ?? configuration.registryPath,
      tmuxSocket: existingSession.tmuxSocket ?? configuration.tmuxSocket
    });
    const options = buildStpTerminalOptions({
      name: session.name,
      workspacePath,
      binaryPath: session.binaryPath ?? configuration.binaryPath,
      registryPath: session.registryPath ?? configuration.registryPath,
      tmuxSocket: session.tmuxSocket ?? configuration.tmuxSocket,
      windowId: randomUUID(),
      terminalId
    });
    return {
      pending: session,
      profile: new vscode2.TerminalProfile({
        ...options,
        shellArgs: [...options.shellArgs]
      })
    };
  } catch (error) {
    if (error instanceof NoWorkspaceError) {
      await vscode2.window.showErrorMessage(error.message);
    }
    throw error;
  }
}

// src/terminalTree.ts
import * as vscode3 from "vscode";

// src/terminalTreeModel.ts
function mergeTerminalTreeItems(openedSessions, registrySessions) {
  const openedIds = new Set(openedSessions.map((session) => session.terminalId));
  const registryItems = registrySessions.filter((session) => !openedIds.has(session.terminalId)).map((session) => ({ kind: "registry", session }));
  return [
    ...openedSessions.map((session) => ({ kind: "opened", session })),
    ...registryItems
  ];
}

// src/terminalTree.ts
class StpTerminalTreeProvider {
  sessions;
  loadRegistrySessions;
  changeEmitter = new vscode3.EventEmitter;
  onDidChangeTreeData = this.changeEmitter.event;
  constructor(sessions, loadRegistrySessions) {
    this.sessions = sessions;
    this.loadRegistrySessions = loadRegistrySessions;
  }
  refresh() {
    this.changeEmitter.fire(undefined);
  }
  getTreeItem(item) {
    const session = item.session;
    const treeItem = new vscode3.TreeItem(session.name, vscode3.TreeItemCollapsibleState.None);
    treeItem.description = session.workspacePath;
    treeItem.contextValue = "stpTerminal";
    treeItem.iconPath = new vscode3.ThemeIcon(item.kind === "opened" ? "terminal" : "plug");
    treeItem.command = {
      command: "stp.showTerminal",
      title: "Show STP Terminal",
      arguments: [item]
    };
    return treeItem;
  }
  getChildren() {
    return mergeTerminalTreeItems(this.sessions.sessions(), this.loadRegistrySessions());
  }
  dispose() {
    this.changeEmitter.dispose();
  }
}

// src/extension.ts
var PROFILE_ID = "stp.terminal";
var SESSIONS_VIEW_ID = "stp.terminals";
var NEW_TERMINAL_COMMAND = "stp.newTerminal";
var SHOW_TERMINAL_COMMAND = "stp.showTerminal";
var TERMINATE_CURRENT_TERMINAL_COMMAND = "stp.terminateCurrentTerminal";
function activate(context) {
  const sessions = new TerminalSessionStore;
  const treeProvider = new StpTerminalTreeProvider(sessions, () => loadLiveRegistrySessions(currentRegistryPath()));
  const provider = {
    async provideTerminalProfile() {
      const { profile } = await createStpTerminalProfile(sessions, currentTerminalProfileConfig());
      return profile;
    }
  };
  context.subscriptions.push(vscode4.window.registerTerminalProfileProvider(PROFILE_ID, provider), vscode4.window.registerTreeDataProvider(SESSIONS_VIEW_ID, treeProvider), vscode4.commands.registerCommand(NEW_TERMINAL_COMMAND, async () => {
    const { pending, profile } = await createStpTerminalProfile(sessions, currentTerminalProfileConfig());
    const terminal = vscode4.window.createTerminal(profile.options);
    const session = sessions.attachOpenedTerminal({
      initialName: pending.name,
      name: pending.name,
      terminal
    });
    if (session !== undefined) {
      treeProvider.refresh();
    }
    terminal.show(false);
    return terminal;
  }), vscode4.commands.registerCommand(SHOW_TERMINAL_COMMAND, (item) => {
    return showStpTerminalTreeItem(sessions, treeProvider, item);
  }), vscode4.commands.registerCommand(TERMINATE_CURRENT_TERMINAL_COMMAND, () => {
    return terminateCurrentStpTerminal(sessions, treeProvider);
  }), vscode4.window.onDidOpenTerminal((terminal) => {
    const initialName = terminalCreationName(terminal);
    const session = sessions.attachOpenedTerminal(initialName === undefined ? { name: terminal.name, terminal } : { initialName, name: terminal.name, terminal });
    if (session !== undefined) {
      treeProvider.refresh();
    }
  }), vscode4.window.onDidCloseTerminal((terminal) => {
    const session = sessions.removeTerminal(terminal);
    if (session !== undefined) {
      terminateClosedStpTerminal(session, treeProvider);
      treeProvider.refresh();
    }
  }), treeProvider);
  cleanupZombieRegistry(treeProvider);
}
function deactivate() {}
function terminateCurrentStpTerminal(sessions, treeProvider) {
  return terminateCurrentTerminal({
    activeTerminal: vscode4.window.activeTerminal,
    binaryPath: currentBinaryPath(),
    messages: {
      showInformationMessage(message) {
        vscode4.window.showInformationMessage(message);
      },
      showErrorMessage(message) {
        vscode4.window.showErrorMessage(message);
      }
    },
    refresh() {
      treeProvider.refresh();
    },
    runner: { run: runStpCommand },
    store: sessions
  });
}
async function showStpTerminalTreeItem(sessions, treeProvider, item) {
  if (item === undefined) {
    return;
  }
  if (item.kind === "opened") {
    showTrackedTerminal(item.session);
    return;
  }
  const openedSession = sessions.sessionForId(item.session.terminalId);
  if (openedSession !== undefined) {
    showTrackedTerminal(openedSession);
    return;
  }
  const { pending, profile } = await createStpTerminalProfile(sessions, currentTerminalProfileConfig(), item.session);
  const terminal = vscode4.window.createTerminal(profile.options);
  const session = sessions.attachOpenedTerminal({
    initialName: pending.name,
    name: pending.name,
    terminal
  });
  if (session !== undefined) {
    treeProvider.refresh();
  }
  terminal.show(false);
}
async function terminateClosedStpTerminal(session, treeProvider) {
  const result = await terminateClosedTerminal({
    binaryPath: currentBinaryPath(),
    runner: { run: runStpCommand },
    session
  });
  if (result.kind === "failed") {
    await vscode4.window.showErrorMessage(`Failed to terminate STP terminal: ${result.message}`);
  }
  treeProvider.refresh();
}
function currentTerminalProfileConfig() {
  return {
    binaryPath: currentBinaryPath(),
    registryPath: currentRegistryPath(),
    tmuxSocket: currentTmuxSocket()
  };
}
async function cleanupZombieRegistry(treeProvider) {
  const result = await cleanupZombieSessions({
    binaryPath: currentBinaryPath(),
    registryPath: currentRegistryPath(),
    runner: { run: runStpCommand }
  });
  if (result.kind === "failed") {
    await vscode4.window.showErrorMessage(`Failed to cleanup zombie STP sessions: ${result.message}`);
  }
  treeProvider.refresh();
}
function terminalCreationName(terminal) {
  const { creationOptions } = terminal;
  return "name" in creationOptions && typeof creationOptions.name === "string" ? creationOptions.name : undefined;
}
export {
  deactivate,
  activate
};

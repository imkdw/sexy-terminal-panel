// src/extension.ts
import { randomUUID } from "node:crypto";
import * as vscode from "vscode";

// src/terminalProfile.ts
class NoWorkspaceError extends Error {
  constructor() {
    super("No workspace folder is open");
    this.name = "NoWorkspaceError";
  }
}
function buildTerminalArgs(input) {
  const args = [
    input.binaryPath,
    "terminal",
    "--workspace",
    input.workspacePath,
    "--window-id",
    input.windowId,
    "--terminal-id",
    input.terminalId
  ];
  if (input.shellPath !== undefined && input.shellPath.length > 0) {
    return [...args, "--shell", input.shellPath];
  }
  return args;
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

// src/extension.ts
var PROFILE_ID = "stp.tmuxTerminal";
var WINDOW_ID_KEY = "stp.windowId";
function activate(context) {
  const provider = {
    async provideTerminalProfile() {
      try {
        const windowId = await windowIdForContext(context);
        const workspacePath = selectWorkspacePath((vscode.workspace.workspaceFolders ?? []).map((folder) => ({
          path: folder.uri.fsPath
        })));
        const binaryPath = selectBinaryPath(vscode.workspace.getConfiguration("stp").inspect("binaryPath"));
        const args = buildTerminalArgs({
          binaryPath,
          workspacePath,
          windowId,
          terminalId: randomUUID()
        });
        const shellPath = args[0];
        if (shellPath === undefined) {
          throw new NoWorkspaceError;
        }
        return new vscode.TerminalProfile({
          name: "STP: tmux",
          shellPath,
          shellArgs: args.slice(1)
        });
      } catch (error) {
        if (error instanceof NoWorkspaceError) {
          await vscode.window.showErrorMessage(error.message);
        }
        throw error;
      }
    }
  };
  context.subscriptions.push(vscode.window.registerTerminalProfileProvider(PROFILE_ID, provider));
}
function deactivate() {}
async function windowIdForContext(context) {
  const existing = context.workspaceState.get(WINDOW_ID_KEY);
  if (existing !== undefined) {
    return existing;
  }
  const next = randomUUID();
  await context.workspaceState.update(WINDOW_ID_KEY, next);
  return next;
}
export {
  deactivate,
  activate
};

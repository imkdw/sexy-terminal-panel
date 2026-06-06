import * as vscode from "vscode"

import { selectRegistryPath } from "./stpRegistry"
import { selectBinaryPath, selectTmuxSocket } from "./terminalProfile"

export function currentBinaryPath(): string {
  return selectBinaryPath(vscode.workspace.getConfiguration("stp").inspect<string>("binaryPath"))
}

export function currentTmuxSocket(): string {
  return selectTmuxSocket(vscode.workspace.getConfiguration("stp").inspect<string>("tmuxSocket"))
}

export function currentRegistryPath(): string {
  return selectRegistryPath(vscode.workspace.getConfiguration("stp").inspect<string>("registryPath"))
}

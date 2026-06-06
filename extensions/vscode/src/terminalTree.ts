import * as vscode from "vscode"

import type { RegistryTerminalSession } from "./stpRegistry"
import type { TerminalSessionStore } from "./terminalSessions"
import { mergeTerminalTreeItems, type StpTerminalTreeItem } from "./terminalTreeModel"

export type RegistrySessionLoader = () => readonly RegistryTerminalSession[]

export class StpTerminalTreeProvider<TTerminal extends vscode.Terminal>
  implements vscode.TreeDataProvider<StpTerminalTreeItem<TTerminal>>, vscode.Disposable
{
  private readonly changeEmitter = new vscode.EventEmitter<
    StpTerminalTreeItem<TTerminal> | undefined
  >()
  readonly onDidChangeTreeData = this.changeEmitter.event

  constructor(
    private readonly sessions: TerminalSessionStore<TTerminal>,
    private readonly loadRegistrySessions: RegistrySessionLoader,
  ) {}

  refresh(): void {
    this.changeEmitter.fire(undefined)
  }

  getTreeItem(item: StpTerminalTreeItem<TTerminal>): vscode.TreeItem {
    const session = item.session
    const treeItem = new vscode.TreeItem(session.name, vscode.TreeItemCollapsibleState.None)
    treeItem.description = session.workspacePath
    treeItem.contextValue = "stpTerminal"
    treeItem.iconPath = new vscode.ThemeIcon(item.kind === "opened" ? "terminal" : "plug")
    treeItem.command = {
      command: "stp.showTerminal",
      title: "Show STP Terminal",
      arguments: [item],
    }
    return treeItem
  }

  getChildren(): vscode.ProviderResult<StpTerminalTreeItem<TTerminal>[]> {
    return mergeTerminalTreeItems(this.sessions.sessions(), this.loadRegistrySessions())
  }

  dispose(): void {
    this.changeEmitter.dispose()
  }
}

export type { StpTerminalTreeItem } from "./terminalTreeModel"

import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

export interface Workspace {
  id: string;
  name: string;
  root: string;
  description?: string;
  source?: string | null;
  createdAt: string;
}

export function workspaceSourceLabel(source?: string | null): string {
  switch (source?.trim().toLowerCase()) {
    case "vscode":
    case "visual studio code":
      return "VS Code";
    case "idea":
    case "intellij":
    case "intellij idea":
      return "IntelliJ IDEA";
    default:
      return source?.trim() ?? "";
  }
}

export function listWorkspaces(): Promise<Workspace[]> {
  return invoke("list_workspaces");
}

export function getCurrentWorkspace(): Promise<Workspace | null> {
  return invoke("get_current_workspace");
}

export function listWorkspaceFiles(): Promise<string[]> {
  return invoke("list_workspace_files");
}

export function createWorkspace(root: string): Promise<Workspace> {
  return invoke("create_workspace", { root });
}

export function switchWorkspace(id: string): Promise<Workspace> {
  return invoke("switch_workspace", { id });
}

export function clearCurrentWorkspace(): Promise<void> {
  return invoke("clear_current_workspace");
}

export function deleteWorkspace(id: string): Promise<void> {
  return invoke("delete_workspace", { id });
}

export async function selectWorkspaceFolder(): Promise<string | null> {
  const selected = await open({ directory: true, multiple: false });
  return typeof selected === "string" ? selected : null;
}

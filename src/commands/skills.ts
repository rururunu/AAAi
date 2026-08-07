import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

export type SkillInfo = {
  name: string;
  source: "builtin" | "user" | string;
  title: string;
  description: string;
  path?: string | null;
  iconUrl?: string | null;
  qualifiedName?: string | null;
  registryId?: string | null;
  namespace?: string | null;
  slug?: string | null;
  homepage?: string | null;
  gitUrl?: string | null;
  verified?: boolean | null;
  categories?: string[] | null;
  origin?: string | null;
};

export function listSkills(): Promise<SkillInfo[]> {
  return invoke("list_skills");
}

export function installSkill(path: string, name?: string): Promise<SkillInfo> {
  return invoke("install_skill", { path, name: name ?? null });
}

export function installSkillMarkdown(
  name: string,
  content: string,
  meta?: Record<string, unknown> | null,
): Promise<SkillInfo> {
  return invoke("install_skill_markdown", { name, content, meta: meta ?? null });
}

export function writeSkillMeta(name: string, meta: Record<string, unknown>): Promise<void> {
  return invoke("write_skill_meta", { name, meta });
}

export function uninstallSkill(name: string): Promise<void> {
  return invoke("uninstall_skill", { name });
}

export function getSkillsDir(): Promise<string> {
  return invoke("get_skills_dir");
}

export function openSkillsDir(): Promise<void> {
  return invoke("open_skills_dir");
}

export async function selectSkillFolder(): Promise<string | null> {
  const selected = await open({ directory: true, multiple: false });
  return typeof selected === "string" ? selected : null;
}

export async function selectSkillFile(): Promise<string | null> {
  const selected = await open({
    directory: false,
    multiple: false,
    filters: [{ name: "Markdown", extensions: ["md"] }],
  });
  return typeof selected === "string" ? selected : null;
}

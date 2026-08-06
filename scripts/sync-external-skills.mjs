#!/usr/bin/env node
/**
 * Sync external document skills into src-tauri/prompts/skills/vendor/
 * for runtime materialization (.aaai/docx, built-in pandoc playbook).
 *
 * Override sources with env:
 *   DOCX_SKILL_SRC  — anthropics docx skill directory
 *   PANDOC_SKILL_SRC — plinde-pandoc pandoc directory (contains SKILL.md)
 */
import { cpSync, existsSync, mkdirSync, rmSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, "..");
const vendorRoot = join(root, "src-tauri", "prompts", "skills", "vendor");

const DOCX_SRC =
  process.env.DOCX_SKILL_SRC ??
  "C:/My/code/github/anthropics_skills/skills/skills/docx";
const PANDOC_SRC =
  process.env.PANDOC_SKILL_SRC ?? "C:/My/code/github/plinde-pandoc/pandoc";

function syncDir(name, src, { excludeSkillMd = false } = {}) {
  const dest = join(vendorRoot, name);
  if (!existsSync(src)) {
    console.error(`[sync-skills] missing source: ${src}`);
    process.exitCode = 1;
    return;
  }
  rmSync(dest, { recursive: true, force: true });
  mkdirSync(vendorRoot, { recursive: true });
  cpSync(src, dest, {
    recursive: true,
    filter: excludeSkillMd
      ? (p) => !p.endsWith("SKILL.md") && !p.endsWith("skill.md")
      : undefined,
  });
  console.log(`[sync-skills] ${name}: ${src} -> ${dest}`);
}

syncDir("docx", DOCX_SRC);
syncDir("pandoc", PANDOC_SRC);

console.log("[sync-skills] done");

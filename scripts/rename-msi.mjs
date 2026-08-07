import { readdir, rename, access } from "node:fs/promises";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

/**
 * Tauri MSI default name: `{product}_{version}_{arch}_{locale}.msi`
 * (e.g. Anya_0.2.1_x64_en-US.msi). Strip the locale suffix for release assets.
 */
const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const msiDir = join(root, "src-tauri", "target", "release", "bundle", "msi");
const localeSuffixMsi = /_([a-z]{2}-[A-Z]{2})\.msi$/i;
const localeSuffixMsiSig = /_([a-z]{2}-[A-Z]{2})\.msi\.sig$/i;

try {
  await access(msiDir);
} catch {
  console.log(`[rename-msi] skip: directory not found (${msiDir})`);
  process.exit(0);
}

const files = await readdir(msiDir);
let renamed = 0;

for (const file of files) {
  let target;
  if (localeSuffixMsi.test(file)) {
    target = file.replace(localeSuffixMsi, ".msi");
  } else if (localeSuffixMsiSig.test(file)) {
    target = file.replace(localeSuffixMsiSig, ".msi.sig");
  } else {
    continue;
  }
  const from = join(msiDir, file);
  const to = join(msiDir, target);

  if (file === target) continue;

  try {
    await access(to);
    console.warn(`[rename-msi] skip: target already exists (${target})`);
    continue;
  } catch {
    // target does not exist — proceed
  }

  await rename(from, to);
  console.log(`[rename-msi] ${file} → ${target}`);
  renamed += 1;
}

if (renamed === 0) {
  console.log("[rename-msi] nothing to rename");
}

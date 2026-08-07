/**
 * Install-scoped icon cache.
 * Browse catalogs use remote URLs directly; only installs call these helpers.
 */

import { convertFileSrc, invoke } from "@tauri-apps/api/core";

export type IconInstallKind = "mcp" | "skill";

/** kind+key → local asset URL */
const memory = new Map<string, string>();
const inflight = new Map<string, Promise<string | null>>();

function memKey(kind: IconInstallKind, cacheKey: string) {
  return `${kind}:${cacheKey}`;
}

export function peekInstallIcon(kind: IconInstallKind, cacheKey: string): string | null {
  return memory.get(memKey(kind, cacheKey)) ?? null;
}

/** Disk lookup only (no network). */
export async function lookupInstallIcon(
  kind: IconInstallKind,
  cacheKey: string,
): Promise<string | null> {
  const key = memKey(kind, cacheKey);
  const hit = memory.get(key);
  if (hit) return hit;
  try {
    const path = await invoke<string | null>("lookup_install_icon", { kind, cacheKey });
    if (!path) return null;
    const local = convertFileSrc(path);
    memory.set(key, local);
    return local;
  } catch {
    return null;
  }
}

/**
 * After install: if not on disk, download `url` and store under the install identity.
 * Returns local asset URL, or null on failure.
 */
export async function cacheInstallIcon(
  kind: IconInstallKind,
  cacheKey: string,
  url: string,
): Promise<string | null> {
  const remote = url.trim();
  if (!remote || !/^https?:\/\//i.test(remote)) return lookupInstallIcon(kind, cacheKey);

  const key = memKey(kind, cacheKey);
  const pending = inflight.get(key);
  if (pending) return pending;

  const task = (async (): Promise<string | null> => {
    try {
      const path = await invoke<string>("cache_install_icon", {
        kind,
        cacheKey,
        url: remote,
      });
      const local = convertFileSrc(path);
      memory.set(key, local);
      return local;
    } catch {
      return null;
    } finally {
      inflight.delete(key);
    }
  })();

  inflight.set(key, task);
  return task;
}

export async function clearInstallIcon(kind: IconInstallKind, cacheKey: string): Promise<void> {
  memory.delete(memKey(kind, cacheKey));
  try {
    await invoke("clear_install_icon", { kind, cacheKey });
  } catch {
    // best-effort
  }
}

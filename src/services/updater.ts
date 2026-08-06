import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export type UpdaterCheckResult =
  | { status: "available"; update: Update; version: string; notes: string }
  | { status: "up-to-date" }
  | { status: "unavailable"; reason: string };

export type UpdaterProgress = {
  phase: "idle" | "checking" | "downloading" | "installing" | "relaunching";
  downloadedBytes: number;
  totalBytes: number;
};

let cachedUpdate: Update | null = null;

function isUpdaterSupported() {
  return import.meta.env.PROD && "__TAURI_INTERNALS__" in window;
}

function normalizeError(error: unknown): string {
  if (error instanceof Error) return error.message;
  return String(error);
}

export async function checkForAppUpdate(): Promise<UpdaterCheckResult> {
  if (!isUpdaterSupported()) {
    return { status: "unavailable", reason: "updater-not-supported" };
  }

  try {
    const update = await check();
    if (!update) {
      cachedUpdate = null;
      return { status: "up-to-date" };
    }

    cachedUpdate = update;
    return {
      status: "available",
      update,
      version: update.version,
      notes: update.body ?? "",
    };
  } catch (error) {
    cachedUpdate = null;
    return { status: "unavailable", reason: normalizeError(error) };
  }
}

export async function installCachedUpdate(
  onProgress?: (progress: UpdaterProgress) => void,
): Promise<void> {
  const update = cachedUpdate;
  if (!update) {
    throw new Error("No update is ready to install.");
  }

  onProgress?.({ phase: "downloading", downloadedBytes: 0, totalBytes: 0 });

  let downloadedBytes = 0;
  let totalBytes = 0;

  await update.downloadAndInstall((event: DownloadEvent) => {
    switch (event.event) {
      case "Started":
        totalBytes = event.data.contentLength ?? 0;
        onProgress?.({ phase: "downloading", downloadedBytes: 0, totalBytes });
        break;
      case "Progress":
        downloadedBytes += event.data.chunkLength;
        onProgress?.({ phase: "downloading", downloadedBytes, totalBytes });
        break;
      case "Finished":
        onProgress?.({ phase: "installing", downloadedBytes, totalBytes });
        break;
    }
  });

  onProgress?.({ phase: "relaunching", downloadedBytes, totalBytes });
  await relaunch();
}

export function clearCachedUpdate() {
  cachedUpdate = null;
}

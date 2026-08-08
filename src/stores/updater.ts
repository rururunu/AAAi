import { defineStore } from "pinia";

import {
  checkForAppUpdate,
  clearCachedUpdate,
  installCachedUpdate,
  type UpdaterProgress,
} from "@/services/updater";

type UpdaterStatus = "idle" | "checking" | "available" | "up-to-date" | "downloading" | "error";

/** How often to re-check while the app stays open. */
const UPDATE_POLL_INTERVAL_MS = 4 * 60 * 60 * 1000;

export const useUpdaterStore = defineStore("updater", {
  state: () => ({
    status: "idle" as UpdaterStatus,
    latestVersion: "",
    releaseNotes: "",
    errorMessage: "",
    progress: {
      phase: "idle",
      downloadedBytes: 0,
      totalBytes: 0,
    } as UpdaterProgress,
    lastCheckedAt: 0,
    pollTimerId: null as ReturnType<typeof setInterval> | null,
  }),

  getters: {
    updateAvailable: (state) => state.status === "available" || state.status === "downloading",
    isBusy: (state) =>
      state.status === "checking" ||
      state.status === "downloading" ||
      state.progress.phase === "installing",
  },

  actions: {
    async check(options: { silent?: boolean } = {}) {
      if (this.status === "checking" || this.status === "downloading") return;
      if (this.progress.phase === "installing") return;

      const silent = Boolean(options.silent);

      // Silent polls must not flash the "checking" UI or clear an available update.
      if (!silent) {
        this.status = "checking";
        this.errorMessage = "";
        this.progress = { phase: "checking", downloadedBytes: 0, totalBytes: 0 };
      }

      const result = await checkForAppUpdate();
      this.lastCheckedAt = Date.now();

      // Ignore late results if install started while we were checking.
      if (this.progress.phase === "downloading" || this.progress.phase === "installing") return;

      if (result.status === "available") {
        this.status = "available";
        this.latestVersion = result.version;
        this.releaseNotes = result.notes;
        this.errorMessage = "";
        if (!silent) {
          this.progress = { phase: "idle", downloadedBytes: 0, totalBytes: 0 };
        }
        return;
      }

      if (result.status === "up-to-date") {
        this.status = "up-to-date";
        this.latestVersion = "";
        this.releaseNotes = "";
        this.errorMessage = "";
        clearCachedUpdate();
        if (!silent) {
          this.progress = { phase: "idle", downloadedBytes: 0, totalBytes: 0 };
        }
        return;
      }

      if (silent) {
        // Keep prior UI + cached update on network blips; don't surface poll errors.
        if (this.status === "checking") this.status = "idle";
        return;
      }

      clearCachedUpdate();
      this.status = "error";
      this.latestVersion = "";
      this.releaseNotes = "";
      this.errorMessage = result.reason;
      this.progress = { phase: "idle", downloadedBytes: 0, totalBytes: 0 };
    },

    async install() {
      if (!this.updateAvailable || this.isBusy) return;

      this.status = "downloading";
      this.errorMessage = "";
      this.progress = { phase: "downloading", downloadedBytes: 0, totalBytes: 0 };

      try {
        await installCachedUpdate((progress) => {
          this.progress = progress;
        });
      } catch (error) {
        this.status = "available";
        this.progress = { phase: "idle", downloadedBytes: 0, totalBytes: 0 };
        this.errorMessage = error instanceof Error ? error.message : String(error);
      }
    },

    /** Start periodic silent checks while the workbench stays open. */
    startPolling() {
      this.stopPolling();
      void this.check({ silent: true });
      this.pollTimerId = setInterval(() => {
        void this.check({ silent: true });
      }, UPDATE_POLL_INTERVAL_MS);
    },

    stopPolling() {
      if (this.pollTimerId != null) {
        clearInterval(this.pollTimerId);
        this.pollTimerId = null;
      }
    },

    resetTransientError() {
      if (this.status === "error") {
        this.status = "idle";
      }
      this.errorMessage = "";
    },
  },
});

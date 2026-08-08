import { defineStore } from "pinia";

import {
  checkForAppUpdate,
  clearCachedUpdate,
  installCachedUpdate,
  type UpdaterProgress,
} from "@/services/updater";

type UpdaterStatus = "idle" | "checking" | "available" | "up-to-date" | "downloading" | "error";

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

      this.status = "checking";
      this.errorMessage = "";
      this.progress = { phase: "checking", downloadedBytes: 0, totalBytes: 0 };

      const result = await checkForAppUpdate();
      this.lastCheckedAt = Date.now();

      if (result.status === "available") {
        this.status = "available";
        this.latestVersion = result.version;
        this.releaseNotes = result.notes;
        return;
      }

      if (result.status === "up-to-date") {
        this.status = "up-to-date";
        this.latestVersion = "";
        this.releaseNotes = "";
        clearCachedUpdate();
        if (!options.silent) {
          this.errorMessage = "";
        }
        return;
      }

      this.status = options.silent ? "idle" : "error";
      this.latestVersion = "";
      this.releaseNotes = "";
      clearCachedUpdate();
      if (!options.silent || result.reason !== "updater-not-supported") {
        this.errorMessage = result.reason;
      }
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

    resetTransientError() {
      if (this.status === "error") {
        this.status = "idle";
      }
      this.errorMessage = "";
    },
  },
});

import { defineStore } from "pinia";

import type { CategoryId } from "@/pages/Settings/settingsDefinitions";

export const useAppStore = defineStore("app", {
  state: () => ({
    dark: true,
    /** Bumped to request the workbench open settings (with category). */
    settingsOpenSignal: 0,
    settingsCategory: "ai" as CategoryId,
  }),
  actions: {
    openSettings(category: CategoryId = "ai") {
      this.settingsCategory = category;
      this.settingsOpenSignal += 1;
    },
  },
});

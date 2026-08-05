import { defineStore } from "pinia";

import { selectDefaultChatModel } from "@/services/chat/ensureDefaultModel";
import { listChatModels } from "@/services/ipc";
import { createLogger } from "@/services/logger";
import type { ChatModelInfo } from "@/types/chat";
import { useSettingStore } from "./setting";

const log = createLogger("chat-model");

export const useChatModelStore = defineStore("chatModel", {
  state: () => ({
    models: [] as ChatModelInfo[],
    loading: false,
    refreshing: false,
    error: null as string | null,
  }),
  actions: {
    async fetch(force = false) {
      if (this.loading) {
        return;
      }

      if (!force && this.models.length > 0) {
        return;
      }

      this.loading = true;
      this.error = null;

      try {
        this.models = await listChatModels();
      } catch (error) {
        log.error("list_chat_models failed", error);
        this.error = error instanceof Error ? error.message : "无法获取模型列表";
      } finally {
        this.loading = false;
      }
    },
    /** Refresh without clearing the list / flashing a loading state. */
    async softRefresh() {
      if (this.loading) {
        return;
      }

      try {
        this.models = await listChatModels();
        this.error = null;
      } catch (error) {
        log.error("list_chat_models failed", error);
        if (this.models.length === 0) {
          this.error = error instanceof Error ? error.message : "无法获取模型列表";
        }
      }
    },
    async refresh() {
      if (this.models.length > 0) {
        await this.reload();
        return;
      }
      await this.fetch(true);
    },
    /** Force reload from API while keeping the current list visible. */
    async reload() {
      if (this.loading || this.refreshing) {
        return;
      }

      this.refreshing = true;

      try {
        this.models = await listChatModels();
        this.error = null;
      } catch (error) {
        log.error("list_chat_models failed", error);
        this.error = error instanceof Error ? error.message : "无法获取模型列表";
      } finally {
        this.refreshing = false;
      }
    },
    /** Pick the first available model when none (or an invalid one) is selected. */
    async ensureDefault(options: { refresh?: boolean } = {}): Promise<ChatModelInfo | null> {
      const settingStore = useSettingStore();

      if (options.refresh) {
        await this.refresh();
      } else if (this.models.length === 0 && !this.loading) {
        await this.fetch();
      }

      const selected = selectDefaultChatModel(
        this.models,
        settingStore.chatModel,
        settingStore.chatModelProvider,
      );
      if (!selected) return null;

      if (selected.needsPersist) {
        await settingStore.update({
          chatModel: selected.model.id,
          chatModelProvider: selected.model.provider,
        });
      }
      return selected.model;
    },
  },
});

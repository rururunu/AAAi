import { defineStore } from "pinia";

import { listChatModels } from "@/services/ipc";
import type { ChatModelInfo } from "@/types/chat";

export const useChatModelStore = defineStore("chatModel", {
    state: () => ({
        models: [] as ChatModelInfo[],
        loading: false,
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
                console.error("list_chat_models failed:", error);
                this.error =
                    error instanceof Error ? error.message : "无法获取模型列表";
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
                console.error("list_chat_models failed:", error);
                if (this.models.length === 0) {
                    this.error =
                        error instanceof Error ? error.message : "无法获取模型列表";
                }
            }
        },
        async refresh() {
            if (this.models.length > 0) {
                await this.softRefresh();
                return;
            }
            await this.fetch(true);
        },
    },
});

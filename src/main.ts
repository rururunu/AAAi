import { createApp } from "vue";
import { createPinia } from "pinia";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import App from "./App.vue";
import router from "./router";
import {
  listenChatContextNotice,
  listenChatDelta,
  listenChatError,
  listenChatFinished,
  listenChatReasoning,
  listenChatStarted,
  listenChatStatus,
  listenChatUserContent,
  listenSettingsChanged,
  listenSettingsOpened,
  listenToolFinished,
  listenToolStarted,
} from "@/services/ipc";
import { normalizeToolActivityEvent, resolveSessionId } from "@/services/chat/normalize";
import { createRafBatch } from "@/services/chat/rafBatch";
import { markPeekWindow } from "@/services/overlay/appearance";
import { installBrowserGuards } from "@/services/browserGuards";
import type {
  ChatContextNoticeEvent,
  ChatDeltaEvent,
  ChatErrorEvent,
  ChatFinishedEvent,
  ChatReasoningEvent,
  ChatStatusEvent,
  ChatUserContentEvent,
} from "@/types/chat";
import { useChatStore } from "@/stores/chat";
import { useSettingStore } from "@/stores/setting";
import "./styles/index.css";

installBrowserGuards();

const app = createApp(App);
const pinia = createPinia();

app.use(pinia);
app.use(router);

const settingStore = useSettingStore();
const chatStore = useChatStore();

type StreamBatchUpdate = {
  sessionId: string;
  messageId: string;
  contentDelta?: string;
  reasoningDelta?: string;
};

const streamBatch = createRafBatch<StreamBatchUpdate>((batch) => {
  chatStore.applyStreamDeltas(
    batch.map((item) => ({
      ...item,
      fallbackSessionId: chatStore.overlayDraftSessionId,
    })),
  );
});

async function bootstrap() {
  const webviewWindow = getCurrentWebviewWindow();
  const windowLabel = webviewWindow.label;
  const isOverlay = (windowLabel === "overlay" || windowLabel.startsWith("overlay-"))
    && !windowLabel.startsWith("overlay-preview-");
  const mountEarly = windowLabel === "workbench" || isOverlay;

  // Resolve each interactive route before loading settings. Mount immediately
  // after the theme is applied, without waiting for optional IPC listeners.
  if (windowLabel === "workbench") {
    void router.replace("/workbench");
  } else if (isOverlay) {
    markPeekWindow();
    void router.replace("/overlay");
  }
  await settingStore.load();
  if (windowLabel === "workbench" || isOverlay) {
    app.mount("#app");
  }

  await listenSettingsChanged((settings) => {
    settingStore.applyPublicSettings(settings);
  });

  await listenChatStarted((payload) => {
    const sId = payload.sessionId;
    if (sId && sId === chatStore.overlayDraftSessionId) {
      chatStore.applyChatStarted(payload);
    }
  });

  await listenChatContextNotice((payload) => {
    const event = payload as ChatContextNoticeEvent & {
      session_id?: string;
    };
    const sId = resolveSessionId(event.sessionId, event.session_id);
    if (sId && (chatStore.sessions[sId] || sId === chatStore.overlayDraftSessionId)) {
      chatStore.setContextNotice(
        sId,
        event.message,
      );
      const prev = chatStore.contextUsage[sId];
      chatStore.setContextUsage(sId, {
        usageRatio: event.usageRatio,
        estimatedTokens: prev?.estimatedTokens ?? 0,
        contextWindowTokens: prev?.contextWindowTokens ?? 0,
      });
    }
  });

  await listenChatDelta((payload) => {
    const event = payload as ChatDeltaEvent & {
      session_id?: string;
      message_id?: string;
    };
    const sId = resolveSessionId(event.sessionId, event.session_id);
    if (sId && (chatStore.sessions[sId] || sId === chatStore.overlayDraftSessionId)) {
      streamBatch.push({
        sessionId: sId,
        messageId: event.messageId ?? event.message_id ?? "",
        contentDelta: event.delta,
      });
    }
  });

  await listenChatReasoning((payload) => {
    const event = payload as ChatReasoningEvent & {
      session_id?: string;
      message_id?: string;
    };
    const sId = resolveSessionId(event.sessionId, event.session_id);
    if (sId && (chatStore.sessions[sId] || sId === chatStore.overlayDraftSessionId)) {
      streamBatch.push({
        sessionId: sId,
        messageId: event.messageId ?? event.message_id ?? "",
        reasoningDelta: event.content,
      });
    }
  });

  await listenChatStatus((payload) => {
    const event = payload as ChatStatusEvent & {
      session_id?: string;
      message_id?: string;
      kind?: string;
    };
    const sId = resolveSessionId(event.sessionId, event.session_id);
    if (sId && (chatStore.sessions[sId] || sId === chatStore.overlayDraftSessionId)) {
      chatStore.setActivityStatus(
        sId,
        event.messageId ?? event.message_id ?? "",
        event.kind ?? "",
        chatStore.overlayDraftSessionId,
      );
    }
  });

  await listenChatUserContent((payload) => {
    const event = payload as ChatUserContentEvent & {
      session_id?: string;
      message_id?: string;
    };
    const sId = resolveSessionId(event.sessionId, event.session_id);
    if (sId && (chatStore.sessions[sId] || sId === chatStore.overlayDraftSessionId)) {
      chatStore.patchMessageContent(
        sId,
        event.messageId ?? event.message_id ?? "",
        event.content,
        chatStore.overlayDraftSessionId,
      );
    }
  });

  await listenChatFinished((payload) => {
    streamBatch.drain();
    const event = payload as ChatFinishedEvent & {
      session_id?: string;
      message_id?: string;
    };
    const sId = resolveSessionId(event.sessionId, event.session_id);
    if (sId && (chatStore.sessions[sId] || sId === chatStore.overlayDraftSessionId)) {
      chatStore.finishMessage(
        sId,
        event.messageId ?? event.message_id ?? "",
        event.content,
        chatStore.overlayDraftSessionId,
        event.reasoning,
      );
    }
  });

  await listenChatError((payload) => {
    streamBatch.drain();
    const event = payload as ChatErrorEvent & {
      session_id?: string;
      message_id?: string;
    };
    const sId = resolveSessionId(event.sessionId, event.session_id);
    if (sId && (chatStore.sessions[sId] || sId === chatStore.overlayDraftSessionId)) {
      chatStore.failMessage(
        sId,
        event.messageId ?? event.message_id ?? "",
        event.message,
        chatStore.overlayDraftSessionId,
      );
    }
  });

  const handleToolActivity = (payload: unknown) => {
    streamBatch.drain();
    const normalized = normalizeToolActivityEvent(
      payload as Parameters<typeof normalizeToolActivityEvent>[0],
    );
    if (!normalized) {
      return;
    }
    const { sessionId, messageId, activity } = normalized;
    if (
      sessionId &&
      (chatStore.sessions[sessionId] || sessionId === chatStore.overlayDraftSessionId)
    ) {
      chatStore.upsertToolActivity(
        sessionId,
        messageId,
        activity,
        chatStore.overlayDraftSessionId,
      );
    }
  };

  await listenToolStarted(handleToolActivity);
  await listenToolFinished(handleToolActivity);

  if (windowLabel.startsWith("overlay-preview-")) {
    document.documentElement.classList.add("peek-window");
    await router.replace("/image-preview");
  } else if (isOverlay) {
    // The overlay route was mounted eagerly above.
  } else if (windowLabel === "settings") {
    await router.replace("/settings");

    await listenSettingsOpened(() => {
      if (router.currentRoute.value.path !== "/settings") {
        void router.replace("/settings");
      }

      const root = document.getElementById("app");
      if (!root?.firstElementChild) {
        globalThis.location.reload();
      }
    });
  }

  await router.isReady();
  if (!mountEarly) {
    app.mount("#app");
  }
}

void bootstrap();

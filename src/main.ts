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
  listenSettingsChanged,
  listenSettingsOpened,
  listenToolFinished,
  listenToolStarted,
} from "@/services/ipc";
import { normalizeToolActivityEvent, resolveSessionId } from "@/services/chat/normalize";
import { createRafBatch } from "@/services/chat/rafBatch";
import { markPeekWindow } from "@/services/overlay/appearance";
import type {
  ChatContextNoticeEvent,
  ChatDeltaEvent,
  ChatErrorEvent,
  ChatFinishedEvent,
  ChatReasoningEvent,
} from "@/types/chat";
import { useChatStore } from "@/stores/chat";
import { useSettingStore } from "@/stores/setting";
import "./services/theme/themes.css";
import "./styles/index.css";

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
  await settingStore.load();

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

  const webviewWindow = getCurrentWebviewWindow();
  const windowLabel = webviewWindow.label;

  if (windowLabel === "overlay" || windowLabel.startsWith("overlay-")) {
    markPeekWindow();
    await router.replace("/overlay");
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
  app.mount("#app");
}

void bootstrap();

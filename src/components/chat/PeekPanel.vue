<template>
  <MotionConfig :transition="springSoft" reduced-motion="user">
    <div
      class="peek-panel"
      :class="{ chat: mode === 'chat', 'minimize-preview': isMinimizePreview }"
      data-tauri-drag-region
      @mousedown="onWindowDragMouseDown"
    >
      <div
        v-show="isMinimizePreview"
        class="minimize-preview-screen"
        data-tauri-drag-region
        aria-hidden="true"
      >
        <span class="minimize-preview-title" data-tauri-drag-region>{{ chatTitle }}</span>
      </div>

      <AnimatePresence mode="popLayout">
        <motion.section
          v-if="mode === 'chat'"
          key="thread"
          class="thread-panel peek-surface"
          :class="{
            glass: isGlass,
            'has-messages': hasVisibleMessages,
          }"
          data-tauri-drag-region
          :initial="false"
          :animate="threadReveal.animate"
          :exit="threadReveal.exit"
          :transition="threadReveal.transition"
        >
          <header class="thread-header" data-tauri-drag-region @mousedown="onWindowDragMouseDown">
            <div
              class="window-controls"
              data-tauri-drag-region="false"
            >
              <button
                type="button"
                class="window-btn btn-minimize"
                :aria-label="tr(settingStore.language, 'minimize')"
                data-tauri-drag-region="false"
                @mousedown.stop.prevent="minimize"
              >
                <Minus :size="12" />
              </button>
              <button
                type="button"
                class="window-btn close"
                :aria-label="tr(settingStore.language, 'close')"
                data-tauri-drag-region="false"
                @mousedown.stop.prevent="close"
              >
                <X :size="12" />
              </button>
            </div>
          </header>

          <p
            v-if="contextNotice"
            class="context-notice"
            data-tauri-drag-region="false"
          >
            {{ contextNotice }}
          </p>

          <MessageList
            :messages="messages"
            :session-id="activeSessionId"
            :checkpoints="checkpoints"
            @rewound="handleRewound"
          />
        </motion.section>
      </AnimatePresence>

      <PlanModeBanner
        :active="planModeActive"
        :language="settingStore.language"
        :busy="planModeBusy"
        @approve="handlePlanApprove"
        @cancel="handlePlanCancel"
      />

      <motion.div
        class="composer-dock peek-surface"
        :class="{ expanded: mode === 'chat', glass: isGlass && mode !== 'chat' }"
        :initial="panelVisible ? dockReveal.initial : false"
        :animate="panelVisible ? dockReveal.animate : dockReveal.initial"
        :exit="dockReveal.exit"
        :transition="dockReveal.transition"
      >
        <p
          v-if="contextPreview"
          class="captured-context-preview"
          data-tauri-drag-region="false"
        >
          {{ contextPreview }}
        </p>
        <ChatInputBar
          ref="inputRef"
          :sending="sending"
          :session-id="activeSessionId"
          :captured-context="capturedContext"
          :placeholder="tr(settingStore.language, mode === 'chat' ? 'continueQuestion' : 'askAnything')"
          :close-on-escape="mode === 'input'"
          :ask-user="askUserSession"
          :path-permission="pathPermissionSession"
          :tool-approval="toolApprovalSession"
          :history-sessions="historySessions"
          :show-workspace-button="mode === 'input'"
          :selection-lines="selectionLines"
          @submit="handleSubmit"
          @pause="handlePause"
          @close="emit('close')"
          @layout-change="handleLayoutChange"
          @ask-user-complete="handleAskUserComplete"
          @path-permission-complete="handlePathPermissionComplete"
          @tool-approval-complete="handleToolApprovalComplete"
          @open-history="handleOpenHistory"
          @history-select="handleHistorySelect"
          @history-close="handleHistoryClose"
          @remove-selection="emit('selectionRemoved')"
          @enter-plan="handleEnterPlan"
        />
      </motion.div>
    </div>
  </MotionConfig>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { storeToRefs } from "pinia";
import { Minus, X } from "@lucide/vue";
import { AnimatePresence, MotionConfig, motion } from "motion-v";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { listen } from "@tauri-apps/api/event";
import ChatInputBar, {
  type AskUserSession,
  type PathPermissionSession,
} from "@/components/chat/ChatInputBar.vue";
import MessageList from "@/components/chat/MessageList.vue";
import PlanModeBanner from "@/components/chat/PlanModeBanner.vue";
import {
  dockReveal,
  springSoft,
  threadReveal,
} from "@/services/motion/presets";
import { refreshOverlayWindowBackground } from "@/services/overlay/appearance";
import { onWindowDragMouseDown } from "@/services/overlay/windowDrag";
import { fetchChatSessions } from "@/commands/slash";
import {
  listenAskUser,
  listenPathPermission,
  listenPlanModeChanged,
  listenToolApproval,
} from "@/services/ipc/events";
import {
  chatCancel,
  getPlanMode,
  listCheckpoints,
  minimizeOverlay,
  respondAskUser,
  respondPathPermission,
  respondToolApproval,
  setOverlayPopupOpen,
  setPlanMode,
} from "@/services/ipc";
import { useChatStore } from "@/stores/chat";
import { useSettingStore } from "@/stores/setting";
import { tr } from "@/services/i18n";
import type {
  AskUserAnswerItem,
  CapturedContext,
  ChatSessionSummary,
  CheckpointInfo,
  PathPermissionDecision,
  ToolApprovalDecision,
  ToolApprovalSession,
} from "@/types/chat";
import { getCurrentWorkspace } from "@/commands/workspace";
import {
  attachSelection,
  parseSelectionAttachment,
  selectionLineCount,
} from "@/services/chat/selectionAttachment";

const props = defineProps<{
  mode: "input" | "chat";
  sessionId: string;
  capturedContext?: CapturedContext | null;
}>();

const emit = defineEmits<{
  layoutChange: [
    payload: {
      showSuggestions: boolean;
      suggestionCount: number;
      showModelMenu: boolean;
      modelMenuHeight: number;
      askUserRowCount: number;
      pickerRowCount: number;
      hasContextPreview: boolean;
      mode: "input" | "chat";
      hasImages?: boolean;
    },
  ];
  close: [];
  enterChat: [sessionId: string];
  contextConsumed: [];
  selectionRemoved: [];
}>();

const chatStore = useChatStore();
const settingStore = useSettingStore();
const { sessions, overlayDraftSessionId, overlayContextNotice } = storeToRefs(chatStore);

const inputRef = ref<InstanceType<typeof ChatInputBar> | null>(null);
const panelVisible = ref(false);
const isMinimizePreview = ref(false);
const MINIMIZE_PREVIEW_MS = 64;
let minimizeTimer: ReturnType<typeof setTimeout> | null = null;
const askUserSession = ref<AskUserSession | null>(null);
const askUserSubmitting = ref(false);
const pathPermissionSession = ref<PathPermissionSession | null>(null);
const pathPermissionSubmitting = ref(false);
const toolApprovalSession = ref<ToolApprovalSession | null>(null);
const toolApprovalSubmitting = ref(false);
const planModeActive = ref(false);
const planModeBusy = ref(false);
const checkpoints = ref<CheckpointInfo[]>([]);
const historySessions = ref<ChatSessionSummary[] | null>(null);
const PUBLIC_HISTORY_LIMIT = 10;

const isGlass = computed(() => settingStore.opacity < 100);
const activeSessionId = computed(
  () => overlayDraftSessionId.value || props.sessionId,
);
const messages = computed(() => {
  const sessionId = activeSessionId.value;
  if (!sessionId) {
    return [];
  }
  return sessions.value[sessionId] ?? [];
});
const hasVisibleMessages = computed(() =>
  messages.value.some(
    (message) => String(message.role).toLowerCase() !== "system",
  ),
);
const sending = computed(() => {
  const sessionId = activeSessionId.value;
  if (!sessionId) {
    return false;
  }
  if (chatStore.sending[sessionId]) {
    return true;
  }
  return messages.value.some(
    (message) =>
      String(message.role).toLowerCase() === "assistant" &&
      (message.status === "pending" || message.status === "streaming"),
  );
});
const contextNotice = computed(() => overlayContextNotice.value);
const selectedText = computed(() => props.capturedContext?.selection?.trim() ?? "");
const selectionLines = computed(() => selectionLineCount(selectedText.value));
const contextPreview = computed(() => {
  const context = props.capturedContext;
  if (!context) {
    return "";
  }

  if (context.selectedFiles?.length) {
    const files = context.selectedFiles;
    const preview =
      files.length === 1
        ? files[0]
        : tr(settingStore.language, "selectedFiles", { file: files[0] ?? "", count: files.length });
    return `[Selected Files] ${preview}`;
  }

  if (context.selectedImages?.length) {
    const count = context.selectedImages.length;
    return count === 1
      ? tr(settingStore.language, "selectedImage")
      : tr(settingStore.language, "selectedImages", { count });
  }

  if (context.activeWindow?.trim()) {
    const firstLine = context.activeWindow.split("\n")[0]?.trim();
    return firstLine ? `[Active Window] ${firstLine}` : "";
  }

  return "";
});
const chatTitle = computed(() => {
  const userMsg = messages.value.find(
    (message) => String(message.role).toLowerCase() === "user",
  );
  const text = userMsg
    ? parseSelectionAttachment(userMsg.content).message.trim()
    : "";
  return text || tr(settingStore.language, "newChat");
});

const composerLayout = ref({
  showSuggestions: false,
  suggestionCount: 0,
  showModelMenu: false,
  modelMenuHeight: 0,
  askUserRowCount: 0,
  pickerRowCount: 0,
  hasImages: false,
});

function emitComposerLayout() {
  emit("layoutChange", {
    ...composerLayout.value,
    hasContextPreview: Boolean(contextPreview.value),
    mode: props.mode,
  });
}

function handleLayoutChange(payload: {
  showSuggestions: boolean;
  suggestionCount: number;
  showModelMenu: boolean;
  modelMenuHeight: number;
  askUserRowCount: number;
  pickerRowCount: number;
  hasImages?: boolean;
}) {
  composerLayout.value = {
    ...payload,
    hasImages: payload.hasImages ?? false,
  };
  emitComposerLayout();
}

function createSessionId() {
  return `session-${Date.now()}`;
}

async function handleSubmit(text: string) {
  const trimmed = text.trim();
  if (!trimmed || sending.value) {
    return;
  }

  if (props.mode === "chat") {
    await chatStore.send(trimmed, activeSessionId.value);
    return;
  }

  const sessionId = createSessionId();
  const messageWithSelection = attachSelection(trimmed, selectedText.value);

  chatStore.setOverlayDraftSession(sessionId);
  chatStore.stageTurn(sessionId, messageWithSelection);
  emit("enterChat", sessionId);
  emit("contextConsumed");

  void chatStore.send(messageWithSelection, sessionId, { staged: true });
}

const activeAssistantMessageId = computed(() => {
  const last = [...messages.value]
    .reverse()
    .find(
      (message) =>
        String(message.role).toLowerCase() === "assistant" &&
        (message.status === "pending" || message.status === "streaming"),
    );
  return last?.id ?? "";
});

async function handlePause() {
  if (!sending.value) {
    return;
  }

  const messageId = activeAssistantMessageId.value;
  const sessionId = activeSessionId.value;
  if (!messageId || !sessionId) {
    return;
  }

  // 乐观恢复发送：后端会再发 chat-finished（cancelled）来对齐状态
  chatStore.clearSending(sessionId);

  try {
    await chatCancel({ messageId });
  } catch (error) {
    console.error("chat_cancel failed:", error);
    // 无活跃任务时（例如异常退出后恢复），本地也要解除卡住的执行态
    chatStore.settleInterruptedSession(sessionId);
  }
}


function close() {
  emit("close");
}

function clearMinimizePreview() {
  isMinimizePreview.value = false;
  if (minimizeTimer) {
    clearTimeout(minimizeTimer);
    minimizeTimer = null;
  }
}

function minimize() {
  if (isMinimizePreview.value) {
    return;
  }

  isMinimizePreview.value = true;
  void nextTick().then(() => {
    minimizeTimer = setTimeout(() => {
      minimizeTimer = null;
      void minimizeOverlay(getCurrentWebviewWindow().label);
    }, MINIMIZE_PREVIEW_MS);
  });
}

async function handleAskUserComplete(answer: string) {
  const session = askUserSession.value;
  if (!session || askUserSubmitting.value) {
    return;
  }
  askUserSubmitting.value = true;

  // 用选择卡片展示回答，不显示 ask_user 原始 JSON
  try {
    const parsed = JSON.parse(answer) as {
      skipped?: boolean;
      answers?: Array<{
        header?: string;
        question?: string;
        selected?: string[];
        userSupplement?: boolean;
      }>;
    };

    const items: AskUserAnswerItem[] =
      parsed.answers
        ?.map((item) => ({
          header: String(item.header ?? "").trim() || undefined,
          selected: (item.selected ?? [])
            .map((v) => String(v).trim())
            .filter(Boolean),
          userSupplement: Boolean(item.userSupplement),
        }))
        .filter(
          (item) => item.userSupplement || item.selected.length > 0,
        ) ?? [];

    if (items.length > 0) {
      chatStore.stageAskUserAnswer(activeSessionId.value, items);
    }
  } catch {
    // ignore formatting errors
  }

  try {
    await respondAskUser({
      requestId: session.requestId,
      answer,
    });
    // 只有后端确认收到回答后，才把 ask_user 工具卡片标为完成/隐藏
    chatStore.completeAskUserToolActivities(activeSessionId.value, answer);
  } catch (error) {
    // 避免工具仍在等待但 UI 已“乐观结束”导致看起来 AI 不回复
    chatStore.stageUserMessage(
      activeSessionId.value,
      tr(settingStore.language, "askSubmitFailed", { error: String(error) }),
    );
    console.error("respond_ask_user failed:", error);
    askUserSubmitting.value = false;
    return;
  }

  const label = getCurrentWebviewWindow().label;
  await setOverlayPopupOpen(label, false);
  askUserSession.value = null;
  emitComposerLayout();
  await nextTick();
  void inputRef.value?.focusInput();
  askUserSubmitting.value = false;
}

function closePathPermission() {
  pathPermissionSession.value = null;
}

async function handleOpenHistory() {
  closeAskUser();
  closePathPermission();
  historySessions.value = await loadScopedHistorySessions();
  const label = getCurrentWebviewWindow().label;
  await setOverlayPopupOpen(label, true);
  void inputRef.value?.focusInput();
}

async function loadScopedHistorySessions() {
  const [allSessions, currentWorkspace] = await Promise.all([
    fetchChatSessions(),
    getCurrentWorkspace(),
  ]);
  if (currentWorkspace) {
    return allSessions.filter(
      (session) => session.workspaceId === currentWorkspace.id,
    );
  }
  return allSessions
    .filter((session) => !session.workspaceId)
    .slice(0, PUBLIC_HISTORY_LIMIT);
}

async function handleHistorySelect(sessionId: string) {
  const label = getCurrentWebviewWindow().label;
  historySessions.value = null;
  await setOverlayPopupOpen(label, false);

  await chatStore.loadHistory(sessionId);
  chatStore.setOverlayDraftSession(sessionId);

  if (props.mode !== "chat") {
    emit("enterChat", sessionId);
  }

  emitComposerLayout();
  await nextTick();
  void inputRef.value?.focusInput();
}

function handleHistoryClose() {
  historySessions.value = null;
  void setOverlayPopupOpen(getCurrentWebviewWindow().label, false);
  emitComposerLayout();
}

async function handlePathPermissionComplete(decision: PathPermissionDecision) {
  const session = pathPermissionSession.value;
  if (!session || pathPermissionSubmitting.value) {
    return;
  }
  pathPermissionSubmitting.value = true;
  try {
    await respondPathPermission({
      requestId: session.requestId,
      decision,
    });
  } catch (error) {
    console.error("respond_path_permission failed:", error);
    pathPermissionSubmitting.value = false;
    return;
  }

  const label = getCurrentWebviewWindow().label;
  await setOverlayPopupOpen(label, false);
  closePathPermission();
  pathPermissionSubmitting.value = false;
  emitComposerLayout();
  await nextTick();
  void inputRef.value?.focusInput();
}

function closeToolApproval() {
  toolApprovalSession.value = null;
}

async function handleToolApprovalComplete(decision: ToolApprovalDecision) {
  const session = toolApprovalSession.value;
  if (!session || toolApprovalSubmitting.value) {
    return;
  }
  toolApprovalSubmitting.value = true;
  try {
    await respondToolApproval({
      requestId: session.requestId,
      decision,
    });
  } catch (error) {
    console.error("respond_tool_approval failed:", error);
    toolApprovalSubmitting.value = false;
    return;
  }
  const label = getCurrentWebviewWindow().label;
  await setOverlayPopupOpen(label, false);
  closeToolApproval();
  toolApprovalSubmitting.value = false;
  emitComposerLayout();
  await nextTick();
  void inputRef.value?.focusInput();
}

async function refreshPlanMode() {
  const sessionId = activeSessionId.value;
  if (!sessionId) {
    planModeActive.value = false;
    return;
  }
  try {
    planModeActive.value = await getPlanMode(sessionId);
  } catch {
    planModeActive.value = false;
  }
}

async function refreshCheckpoints() {
  const sessionId = activeSessionId.value;
  if (!sessionId) {
    checkpoints.value = [];
    return;
  }
  try {
    checkpoints.value = await listCheckpoints(sessionId);
  } catch {
    checkpoints.value = [];
  }
}

async function handleEnterPlan() {
  const sessionId = activeSessionId.value;
  if (!sessionId) {
    return;
  }
  planModeBusy.value = true;
  try {
    await setPlanMode(sessionId, true);
    planModeActive.value = true;
  } catch (error) {
    console.error("set_plan_mode failed:", error);
  } finally {
    planModeBusy.value = false;
  }
}

async function handlePlanApprove() {
  const sessionId = activeSessionId.value;
  if (!sessionId) {
    return;
  }
  planModeBusy.value = true;
  try {
    await setPlanMode(sessionId, false);
    planModeActive.value = false;
  } catch (error) {
    console.error("approve plan failed:", error);
  } finally {
    planModeBusy.value = false;
  }
}

async function handlePlanCancel() {
  await handlePlanApprove();
}

async function handleRewound(payload: { text: string }) {
  await chatStore.loadHistory(activeSessionId.value);
  await refreshCheckpoints();
  if (payload.text) {
    inputRef.value?.setMessage(payload.text);
  } else {
    void inputRef.value?.focusInput();
  }
}

function closeAskUser() {
  askUserSession.value = null;
  void setOverlayPopupOpen(getCurrentWebviewWindow().label, false);
}

watch(
  () => activeSessionId.value,
  () => {
    void refreshPlanMode();
    void refreshCheckpoints();
  },
);

watch(
  () =>
    messages.value
      .map((message) => `${message.id}:${message.status}:${message.toolActivities?.length ?? 0}`)
      .join("|"),
  () => {
    void refreshCheckpoints();
  },
);

watch(
  () => [contextPreview.value, props.mode, askUserSession.value] as const,
  () => {
    emitComposerLayout();
  },
);

watch(
  () => props.mode,
  (mode) => {
    if (mode === "chat") {
      void inputRef.value?.focusInput();
    }
    emitComposerLayout();
  },
);

onMounted(async () => {
  const window = getCurrentWebviewWindow();

  void listenAskUser(async (payload) => {
    if (payload.sessionId && payload.sessionId !== activeSessionId.value) {
      return;
    }
    pathPermissionSession.value = null;
    toolApprovalSession.value = null;
    askUserSession.value = {
      requestId: payload.requestId,
      questions: payload.questions,
    };
    const label = getCurrentWebviewWindow().label;
    await setOverlayPopupOpen(label, true);
    void inputRef.value?.focusInput();
  });

  void listenPathPermission(async (payload) => {
    if (payload.sessionId && payload.sessionId !== activeSessionId.value) {
      return;
    }
    askUserSession.value = null;
    toolApprovalSession.value = null;
    pathPermissionSession.value = {
      requestId: payload.requestId,
      path: payload.path,
      operation: payload.operation,
      toolName: payload.toolName,
    };
    const label = getCurrentWebviewWindow().label;
    await setOverlayPopupOpen(label, true);
    void inputRef.value?.focusInput();
  });

  void listenToolApproval(async (payload) => {
    if (payload.sessionId && payload.sessionId !== activeSessionId.value) {
      return;
    }
    askUserSession.value = null;
    pathPermissionSession.value = null;
    toolApprovalSession.value = {
      requestId: payload.requestId,
      toolName: payload.toolName,
      title: payload.title,
      preview: payload.preview ?? null,
    };
    chatStore.attachToolApprovalPreview(
      payload.sessionId || activeSessionId.value,
      payload.toolName,
      payload.preview ?? null,
      activeSessionId.value,
    );
    const label = getCurrentWebviewWindow().label;
    await setOverlayPopupOpen(label, true);
    void inputRef.value?.focusInput();
  });

  void listenPlanModeChanged((payload) => {
    if (payload.sessionId && payload.sessionId !== activeSessionId.value) {
      return;
    }
    planModeActive.value = payload.active;
  });

  await window.listen("overlay-shown", () => {
    clearMinimizePreview();
    void refreshOverlayWindowBackground();
    panelVisible.value = true;
    void inputRef.value?.focusInput();
  });

  await window.listen("overlay-hidden", () => {
    clearMinimizePreview();
    panelVisible.value = false;
    inputRef.value?.reset();
    emit("layoutChange", {
      showSuggestions: false,
      suggestionCount: 0,
      showModelMenu: false,
      modelMenuHeight: 0,
      askUserRowCount: 0,
      pickerRowCount: 0,
      hasContextPreview: false,
      mode: props.mode,
    });
    closeAskUser();
    closePathPermission();
    closeToolApproval();
    historySessions.value = null;
  });

  void refreshPlanMode();
  void refreshCheckpoints();

  if (await window.isVisible()) {
    panelVisible.value = true;
    // 动态新建窗口时，overlay-shown 在 Vue 挂载前就发出了，
    // 这里补做相同的初始化：刷新背景透明度、聚焦输入框
    void refreshOverlayWindowBackground();
    void inputRef.value?.focusInput();
  } else {
    void inputRef.value?.focusInput();
  }

  void listen<string>("open-session", async (event) => {
    const targetSessionId = event.payload;
    await handleHistorySelect(targetSessionId);
  });

  void listen("history-updated", async () => {
    if (historySessions.value !== null) {
      historySessions.value = await loadScopedHistorySessions();
    }
  });

  void listen<{ sessionId?: string; command?: string; args?: string }>(
    "slash-command",
    (event) => {
      const command = (event.payload?.command ?? "").replace(/^\//, "").toLowerCase();
      if (props.sessionId && event.payload?.sessionId && event.payload.sessionId !== props.sessionId) {
        return;
      }
      switch (command) {
        case "history":
          historySessions.value = historySessions.value ?? [];
          void loadScopedHistorySessions().then((sessions) => {
            historySessions.value = sessions;
          });
          break;
        case "plan":
          void handleEnterPlan();
          break;
        case "work":
          void inputRef.value?.focusInput();
          break;
        case "exit":
          emit("close");
          break;
        case "clear":
          inputRef.value?.reset();
          break;
        default:
          break;
      }
    },
  );
});

onUnmounted(() => {
  clearMinimizePreview();
});
</script>

<style scoped>
.peek-panel {
  box-sizing: border-box;
  width: 100%;
  height: 100%;
  position: relative;
  display: flex;
  flex-direction: column;
  justify-content: flex-end;
  align-items: stretch;
  background: transparent;
  color: var(--peek-text);
  overflow: hidden;
  border: none;
  outline: none;
  --thread-side-gap: 14px;
}

.peek-panel.chat {
  --composer-overlap: 12px;
  --composer-clearance: 90px;
}

.thread-panel {
  flex: 1;
  min-height: 0;
  min-width: 0;
  width: calc(100% - (2 * var(--thread-side-gap)));
  margin: 0 auto calc(-1 * var(--composer-overlap, 12px));
  display: flex;
  flex-direction: column;
  border: none;
  border-radius: 8px 8px 0 0;
  background: color-mix(in srgb, var(--peek-list-bg) 92%, transparent);
  overflow: hidden;
  position: relative;
  z-index: 1;
  isolation: isolate;
}

.thread-panel.glass {
  background: color-mix(in srgb, var(--peek-list-bg) 76%, transparent);
  backdrop-filter: blur(20px) saturate(1.15);
  -webkit-backdrop-filter: blur(20px) saturate(1.15);
}

.peek-panel.chat :deep(.message-list) {
  position: relative;
  z-index: 2;
  padding-top: 42px;
  scroll-padding-top: 42px;
  padding-bottom: calc(
    var(--composer-overlap, 12px) + var(--composer-clearance, 90px)
  );
}

.thread-header {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  z-index: 5;
  height: 34px;
  display: flex;
  align-items: center;
  justify-content: flex-end;
  padding: 0 8px;
  border-bottom: 1px solid transparent;
  background: transparent;
  opacity: 0;
  pointer-events: auto;
  transition:
    opacity 160ms ease,
    border-color 160ms ease,
    background 160ms ease;
  cursor: grab;
}

.thread-header:hover {
  opacity: 1;
  border-bottom-color: var(--peek-border);
  background: color-mix(in srgb, var(--peek-sidebar) 88%, transparent);
}

.thread-panel.glass .thread-header:hover {
  backdrop-filter: blur(16px) saturate(1.1);
  -webkit-backdrop-filter: blur(16px) saturate(1.1);
}

.thread-header:active {
  cursor: grabbing;
}

/* 红色：全宽消息框底座 */
.composer-dock {
  flex: none;
  width: 100%;
  border: none;
  border-radius: 8px;
  background: var(--peek-surface);
  position: relative;
  z-index: 2;
  overflow: hidden;
  isolation: isolate;
}

.composer-dock :deep(.chat-input-shell) {
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.composer-dock.expanded {
  border: none;
  border-radius: 8px;
  background: linear-gradient(
    180deg,
    color-mix(in srgb, var(--peek-surface) 88%, var(--peek-list-bg)) 0%,
    var(--peek-surface) 22%,
    var(--peek-surface) 100%
  );
}

.peek-panel.chat :deep(.input-footer-primary) {
  flex-wrap: wrap;
  row-gap: 4px;
}

.window-controls {
  flex: none;
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.window-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  margin: 0;
  padding: 0;
  border: 1px solid transparent;
  border-radius: 50%;
  background: transparent;
  color: var(--peek-muted);
  cursor: default;
  transition: background 120ms ease, color 120ms ease, border-color 120ms ease;
}

.window-btn:hover {
  background: var(--peek-list-active);
  color: var(--peek-text);
  border-color: color-mix(in srgb, var(--peek-accent) 24%, var(--peek-border));
}

.window-btn.close:hover {
  background: var(--destructive);
  border-color: var(--destructive);
  color: #fff;
}

.context-notice {
  flex: none;
  margin: 0;
  padding: 6px 12px;
  font-size: 11px;
  line-height: 1.4;
  color: var(--peek-muted);
  background: color-mix(in srgb, var(--peek-accent) 8%, transparent);
  border-bottom: 1px solid color-mix(in srgb, var(--peek-accent) 16%, var(--peek-border));
}

.captured-context-preview {
  flex: none;
  margin: 0;
  padding: 8px 12px 4px;
  font-size: 11px;
  line-height: 1.45;
  color: var(--peek-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  border-bottom: 1px solid color-mix(in srgb, var(--peek-border) 70%, transparent);
}

.peek-panel.minimize-preview .thread-panel,
.peek-panel.minimize-preview .composer-dock,
.peek-panel.minimize-preview .plan-mode-banner {
  visibility: hidden;
  pointer-events: none;
}

.minimize-preview-screen {
  position: absolute;
  inset: 0;
  z-index: 30;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0 20px;
  background: var(--peek-bg);
  color: var(--peek-text);
  box-sizing: border-box;
  user-select: none;
}

.minimize-preview-title {
  width: 100%;
  min-width: 0;
  font-size: 17px;
  font-weight: 600;
  line-height: 1.4;
  text-align: center;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

</style>

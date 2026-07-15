<template>
  <MotionConfig :transition="springSoft" reduced-motion="user">
    <div
      class="peek-panel"
      :class="{ chat: mode === 'chat', collapsed: isCollapsed }"
      data-tauri-drag-region
      @mousedown="onWindowDragMouseDown"
    >
      <div v-if="isCollapsed" class="capsule-container" data-tauri-drag-region>
        <div class="capsule-content" data-tauri-drag-region>
          <div class="capsule-spark-wrapper" data-tauri-drag-region>
            <svg class="capsule-spark-icon" viewBox="0 0 16 16" fill="none" data-tauri-drag-region>
              <path
                d="M8 2.25L9.35 6.15L13.25 7.5L9.35 8.85L8 12.75L6.65 8.85L2.75 7.5L6.65 6.15L8 2.25Z"
                stroke="currentColor"
                stroke-width="1.35"
                stroke-linejoin="round"
                data-tauri-drag-region
              />
            </svg>
          </div>
          <span class="capsule-badge" data-tauri-drag-region>PEEK</span>
          <span class="capsule-divider" data-tauri-drag-region></span>
          <span class="capsule-title" data-tauri-drag-region>{{ chatTitle }}</span>
        </div>
        <div class="capsule-actions" data-no-drag>
          <button
            type="button"
            class="capsule-action-btn expand"
            :aria-label="tr(settingStore.language, 'expand')"
            @click.stop="expand"
          >
            <ChevronUp :size="10" />
          </button>
          <button
            type="button"
            class="capsule-action-btn close"
            :aria-label="tr(settingStore.language, 'close')"
            @click.stop="close"
          >
            <X :size="10" />
          </button>
        </div>
      </div>

      <template v-else>
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
              <motion.button
                type="button"
                class="window-btn btn-collapse"
                :aria-label="tr(settingStore.language, 'collapse')"
                data-tauri-drag-region="false"
                :while-hover="{ scale: 1.08 }"
                :while-press="{ scale: 0.92 }"
                @mousedown.prevent="collapse"
              >
                <ChevronDown :size="12" />
              </motion.button>
              <motion.button
                type="button"
                class="window-btn close"
                :aria-label="tr(settingStore.language, 'close')"
                data-tauri-drag-region="false"
                :while-hover="{ scale: 1.08 }"
                :while-press="{ scale: 0.92 }"
                @mousedown.prevent="close"
              >
                <X :size="12" />
              </motion.button>
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
      </template>
    </div>
  </MotionConfig>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { storeToRefs } from "pinia";
import { ChevronDown, ChevronUp, X } from "@lucide/vue";
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
    },
  ];
  close: [];
  enterChat: [sessionId: string];
  contextConsumed: [];
  selectionRemoved: [];
  collapse: [];
  expand: [];
}>();

const chatStore = useChatStore();
const settingStore = useSettingStore();
const { sessions, overlayDraftSessionId, overlayContextNotice } = storeToRefs(chatStore);

const inputRef = ref<InstanceType<typeof ChatInputBar> | null>(null);
const panelVisible = ref(false);
const isCollapsed = ref(false);
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
  return userMsg ? parseSelectionAttachment(userMsg.content).message : tr(settingStore.language, "newChat");
});

const composerLayout = ref({
  showSuggestions: false,
  suggestionCount: 0,
  showModelMenu: false,
  modelMenuHeight: 0,
  askUserRowCount: 0,
  pickerRowCount: 0,
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
}) {
  composerLayout.value = payload;
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

function collapse() {
  isCollapsed.value = true;
  emit("collapse");
}

function expand() {
  isCollapsed.value = false;
  emit("expand");
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
  isCollapsed.value = false;
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
    isCollapsed.value = false;
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
    isCollapsed.value = false;
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
    isCollapsed.value = false;
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
    void refreshOverlayWindowBackground();
    panelVisible.value = true;
    void inputRef.value?.focusInput();
  });

  await window.listen("overlay-hidden", () => {
    panelVisible.value = false;
    isCollapsed.value = false;
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
    if (isCollapsed.value) {
      expand();
    }
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
</script>

<style scoped>
.peek-panel {
  box-sizing: border-box;
  width: 100%;
  height: 100%;
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

.peek-panel.collapsed {
  justify-content: center;
  align-items: center;
  padding: 0;
}

.capsule-container {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  height: 100%;
  padding: 0 12px;
  border-radius: 9999px;
  background: color-mix(in srgb, var(--peek-surface) 80%, rgba(10, 10, 10, 0.4));
  border: 1px solid color-mix(in srgb, var(--peek-border) 40%, rgba(255, 255, 255, 0.05));
  box-shadow: 
    0 4px 16px rgba(0, 0, 0, 0.25),
    inset 0 1px 1px rgba(255, 255, 255, 0.08);
  backdrop-filter: blur(24px) saturate(1.2);
  -webkit-backdrop-filter: blur(24px) saturate(1.2);
  cursor: pointer;
  user-select: none;
  box-sizing: border-box;
  transition: all 250ms cubic-bezier(0.16, 1, 0.3, 1);
  overflow: hidden;
}

.capsule-container:hover {
  background: color-mix(in srgb, var(--peek-surface) 90%, rgba(15, 15, 15, 0.5));
  border-color: color-mix(in srgb, var(--peek-accent) 40%, rgba(255, 255, 255, 0.15));
  box-shadow: 
    0 6px 20px rgba(0, 0, 0, 0.35),
    inset 0 1px 1.5px rgba(255, 255, 255, 0.12);
  transform: translateY(-0.5px);
}

.capsule-content {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 8px;
}

.capsule-spark-wrapper {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: color-mix(in srgb, var(--peek-accent) 12%, transparent);
  color: var(--peek-accent);
  flex-shrink: 0;
  cursor: grab;
}

.capsule-spark-icon {
  width: 12px;
  height: 12px;
}

.capsule-badge {
  font-family: var(--font-sans);
  font-size: 9px;
  font-weight: 700;
  letter-spacing: 0.08em;
  color: var(--peek-accent);
  opacity: 0.85;
  flex-shrink: 0;
  cursor: grab;
}

.capsule-divider {
  width: 1px;
  height: 10px;
  background: color-mix(in srgb, var(--peek-border) 60%, transparent);
  flex-shrink: 0;
}

.capsule-title {
  flex: 1;
  min-width: 0;
  font-size: 12px;
  font-weight: 500;
  color: var(--peek-text);
  opacity: 0.9;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}

.capsule-actions {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
  margin-left: 6px;
}

.capsule-action-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: transparent;
  color: var(--peek-muted);
  border: none;
  cursor: pointer;
  opacity: 0;
  transform: scale(0.8);
  transition: all 200ms cubic-bezier(0.16, 1, 0.3, 1);
  flex-shrink: 0;
}

.capsule-container:hover .capsule-action-btn {
  opacity: 1;
  transform: scale(1);
}

.capsule-action-btn:hover {
  background: color-mix(in srgb, var(--peek-accent) 15%, rgba(255, 255, 255, 0.1));
  color: var(--peek-text);
}

.capsule-action-btn.close:hover {
  background: var(--destructive);
  color: #fff;
}

</style>

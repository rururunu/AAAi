<template>
  <div class="message-list-shell">
    <nav v-if="userMessages.length" class="message-preview-rail" :aria-label="tr(settingStore.language, 'userMessageNav')">
      <button
        v-for="(message, index) in userMessages"
        :key="message.id"
        type="button"
        class="message-preview-mark"
        :class="{ active: message.id === activeUserMessageId }"
        :aria-label="tr(settingStore.language, 'jumpMessage', { count: index + 1 })"
        @click="scrollToMessage(message.id)"
      >
        <span class="mark-line" aria-hidden="true"></span>
        <span class="message-preview-tooltip">{{ messagePreview(message) }}</span>
      </button>
    </nav>

    <div
      ref="listRef"
      class="message-list peek-scrollbar"
      data-tauri-drag-region="false"
      @scroll="handleScroll"
    >
      <article
        v-for="message in visibleMessages"
        :key="message.id"
        class="message-item"
        :class="messageRoleClass(message)"
        :data-message-id="message.id"
      >
        <div v-if="isUserMessage(message)" class="user-turn">
          <div
            v-if="userContent(message).images?.length"
            class="user-images"
            data-tauri-drag-region="false"
          >
            <button
              v-for="(img, idx) in userContent(message).images"
              :key="idx"
              type="button"
              class="user-image-btn"
              data-tauri-drag-region="false"
              data-no-drag
              :aria-label="'Preview image'"
              @mousedown.stop
              @click.stop.prevent="previewImage(img)"
            >
              <img :src="img" class="user-image" alt="" draggable="false" />
            </button>
          </div>
          <div
            v-if="userContent(message).message || userContent(message).selection"
            class="user-bubble"
          >
            <span v-if="userContent(message).message" class="user-message-text">{{ userContent(message).message }}</span>
            <span v-if="userContent(message).selection" class="user-selection-quote">
              {{ userContent(message).selection }}
            </span>
          </div>
          <button
            v-if="checkpointFor(message)"
            type="button"
            class="rewind-icon-btn"
            :disabled="rewindBusy"
            :aria-label="tr(settingStore.language, 'rewind')"
            :title="tr(settingStore.language, 'rewind')"
            @click.stop="confirmRewind(message)"
          >
            <Undo2 :size="14" :stroke-width="2" aria-hidden="true" />
          </button>
        </div>
        <div v-else class="assistant-bubble">
          <AgentWorkDetails :message="message" :language="settingStore.language" />
          <AskUserAnswerCard v-if="message.askUserAnswer?.length" :items="message.askUserAnswer" />
          <ImageAnalysisDetails
            v-for="(analysis, idx) in imageAnalysesForAssistant(message)"
            :key="`${message.id}-analysis-${idx}`"
            :model="analysis.model"
            :text="analysis.text"
          />
          <Markdown v-if="message.content" :content="message.content" />
          <AssistantActivityIndicator
            v-if="activityLabel(message)"
            :label="activityLabel(message)!"
          />
        </div>
      </article>
    </div>

    <AppConfirmDialog ref="confirmDialogRef" />
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { Undo2 } from "@lucide/vue";
import AgentWorkDetails from "@/components/chat/AgentWorkDetails.vue";
import AssistantActivityIndicator from "@/components/chat/AssistantActivityIndicator.vue";
import AskUserAnswerCard from "@/components/chat/AskUserAnswerCard.vue";
import ImageAnalysisDetails from "@/components/chat/ImageAnalysisDetails.vue";
import Markdown from "@/components/chat/Markdown.vue";
import { AppConfirmDialog } from "@/components/ui/confirm-dialog";
import { rewindSession, openImagePreview } from "@/services/ipc";
import { useSettingStore } from "@/stores/setting";
import type { ChatMessage, CheckpointInfo } from "@/types/chat";
import { parseSelectionAttachment } from "@/services/chat/selectionAttachment";
import { tr } from "@/services/i18n";
function previewImage(url: string) {
  void openImagePreview(url).catch((error) => {
    console.error("openImagePreview failed:", error);
  });
}

const SCROLL_NEAR_BOTTOM_THRESHOLD = 96;
const props = defineProps<{
  messages: ChatMessage[];
  sessionId?: string;
  checkpoints?: CheckpointInfo[];
}>();
const emit = defineEmits<{
  rewound: [payload: { text: string }];
}>();
const settingStore = useSettingStore();
const confirmDialogRef = ref<InstanceType<typeof AppConfirmDialog> | null>(null);
const visibleMessages = computed(() =>
  props.messages.filter((message) => {
    const role = String(message.role).toLowerCase();
    return role !== "system" && role !== "tool";
  }),
);
const userMessages = computed(() => visibleMessages.value.filter(isUserMessage));
const listRef = ref<HTMLElement | null>(null);
const stickToBottom = ref(true);
const activeUserMessageId = ref("");
const rewindBusy = ref(false);

function normalizeRole(role: ChatMessage["role"] | string) {
  return String(role).toLowerCase();
}
function isUserMessage(message: ChatMessage) {
  return normalizeRole(message.role) === "user";
}
function userContent(message: ChatMessage) {
  return parseSelectionAttachment(message.content);
}

/** Image analyses are persisted on the preceding user message; show them on the assistant turn. */
function precedingUserMessage(assistant: ChatMessage): ChatMessage | undefined {
  const list = visibleMessages.value;
  const index = list.findIndex((item) => item.id === assistant.id);
  if (index <= 0) return undefined;
  for (let i = index - 1; i >= 0; i -= 1) {
    if (isUserMessage(list[i]!)) {
      return list[i];
    }
  }
  return undefined;
}

function imageAnalysesForAssistant(message: ChatMessage) {
  const user = precedingUserMessage(message);
  if (!user) return [];
  return userContent(user).imageAnalyses ?? [];
}

function messageRoleClass(message: ChatMessage) {
  return isUserMessage(message) ? "user" : "assistant";
}
function checkpointFor(message: ChatMessage) {
  return (props.checkpoints ?? []).find((item) => item.userMessageId === message.id);
}
async function confirmRewind(message: ChatMessage) {
  const checkpoint = checkpointFor(message);
  if (!checkpoint || !props.sessionId || rewindBusy.value) return;

  const confirmed = await confirmDialogRef.value?.ask({
    title: tr(settingStore.language, "rewindConfirmTitle"),
    description: tr(settingStore.language, "rewindConfirm"),
    confirmLabel: tr(settingStore.language, "rewindConfirmAction"),
    cancelLabel: tr(settingStore.language, "rewindCancel"),
  });
  if (!confirmed) return;

  const text = userContent(message).message.trim();
  rewindBusy.value = true;
  try {
    await rewindSession({
      sessionId: props.sessionId,
      turn: checkpoint.turn,
      restore: "both",
    });
    emit("rewound", { text });
  } catch (error) {
    console.error("rewind_session failed:", error);
  } finally {
    rewindBusy.value = false;
  }
}
function isPending(message: ChatMessage) {
  return message.status === "pending" || message.status === "streaming";
}
function getFilename(path: string | undefined): string {
  if (!path) return "";
  const parts = path.split(/[/\\]/);
  return parts[parts.length - 1] || path;
}

function activityLabel(message: ChatMessage) {
  if (!isPending(message) || message.askUserAnswer?.length) return "";

  // Prefer real reply progress over a stale analyzing label.
  if (
    message.activityStatus === "analyzing_images" &&
    !message.content.trim() &&
    !message.reasoning?.trim()
  ) {
    return tr(settingStore.language, "analyzingImages");
  }

  const running = [...(message.toolActivities ?? [])]
    .reverse()
    .find((activity) => activity.status === "running");
  if (running) {
    if (running.toolName === "ask_user") return "";
    
    const args = (running.arguments || {}) as Record<string, any>;
    
    // 1. Reading file
    if (running.toolName === "read_file" || running.toolName === "view_file") {
      const path = args.AbsolutePath || args.TargetFile || args.path;
      const file = getFilename(path);
      return file ? `正在读取 ${file}` : "正在读取文件";
    }
    
    // 2. Listing directory / Getting workspace details
    if (
      running.toolName === "list_dir" || 
      running.toolName === "list_folder" ||
      running.toolName === "list_workspace_files"
    ) {
      return "正在获取目录信息";
    }
    
    // 3. Writing or editing file
    if (
      running.toolName === "write_to_file" || 
      running.toolName === "replace_file_content" || 
      running.toolName === "multi_replace_file_content" ||
      ["create", "edit", "delete", "move"].includes(running.kind)
    ) {
      const path = args.TargetFile || args.AbsolutePath || args.path || args.to || args.from;
      const file = getFilename(path);
      return file ? `正在编写 ${file}` : "正在编写中";
    }
    
    // 4. Searching / Grep search
    if (
      running.toolName === "grep_search" || 
      running.toolName === "find_files" || 
      running.toolName === "search_files"
    ) {
      const query = args.Query || args.pattern || args.query;
      return query ? `正在查找 "${query}"` : "正在查找";
    }
    
    // 5. Web Search / Read URL
    if (running.toolName === "search_web") {
      const query = args.query;
      return query ? `正在搜索 "${query}"` : "正在进行网页搜索";
    }
    if (running.toolName === "read_url_content") {
      return "正在读取网页内容";
    }
    
    // 6. Shell Command
    if (running.toolName === "run_command" || running.kind === "shell") {
      const cmd = args.CommandLine || args.command || args.commandLine;
      return cmd ? `正在执行: ${cmd}` : "正在执行命令";
    }

    if (running.kind === "read") return tr(settingStore.language, "reading");
    return tr(settingStore.language, "working");
  }
  if (message.content) return tr(settingStore.language, "responding");
  return tr(settingStore.language, "thinking");
}
function isNearBottom(element: HTMLElement) {
  return element.scrollHeight - element.scrollTop - element.clientHeight <= SCROLL_NEAR_BOTTOM_THRESHOLD;
}
function handleScroll() {
  const element = listRef.value;
  if (!element) return;
  stickToBottom.value = isNearBottom(element);
  updateActiveUserMessage();
}
function updateActiveUserMessage() {
  const element = listRef.value;
  if (!element) return;
  const top = element.getBoundingClientRect().top + 24;
  let active = userMessages.value[0]?.id ?? "";
  for (const message of userMessages.value) {
    const node = element.querySelector<HTMLElement>(`[data-message-id="${CSS.escape(message.id)}"]`);
    if (node && node.getBoundingClientRect().top <= top) active = message.id;
  }
  activeUserMessageId.value = active;
}
function scrollToMessage(messageId: string) {
  const node = listRef.value?.querySelector<HTMLElement>(`[data-message-id="${CSS.escape(messageId)}"]`);
  if (!node) return;
  stickToBottom.value = false;
  activeUserMessageId.value = messageId;
  node.scrollIntoView({ behavior: "smooth", block: "start" });
}
function messagePreview(message: ChatMessage) {
  const compact = userContent(message).message.replace(/\s+/g, " ").trim();
  return compact.length > 72 ? `${compact.slice(0, 72)}...` : compact;
}
async function scrollToBottomIfNeeded() {
  await nextTick();
  const element = listRef.value;
  if (!element || !stickToBottom.value) return;
  element.scrollTop = element.scrollHeight;
  updateActiveUserMessage();
}

watch(
  () => props.messages.length,
  (length, previousLength) => {
    if (length > (previousLength ?? 0)) stickToBottom.value = true;
  },
);
watch(
  () => props.messages.map((item) => `${item.id}:${item.content.length}:${item.reasoning?.length ?? 0}:${item.askUserAnswer?.map((a) => a.selected.join(",")).join(";") ?? ""}:${item.toolActivities?.map((activity) => `${activity.id}:${activity.status}:${activity.detail?.length ?? 0}`).join(",") ?? ""}:${item.status}:${item.activityStatus ?? ""}`).join("|"),
  () => void scrollToBottomIfNeeded(),
  { immediate: true },
);
</script>

<style scoped>
.message-list-shell { position: relative; display: flex; flex: 1; min-height: 0; }
.message-list { flex: 1; min-height: 0; overflow-x: hidden; overflow-y: auto; overscroll-behavior: contain; padding: 12px 28px 10px 12px; display: flex; flex-direction: column; gap: 14px; scroll-padding-top: 12px; }
.message-preview-rail { position: absolute; z-index: 4; top: 42px; right: 7px; bottom: 10px; display: flex; flex-direction: column; gap: 5px; width: 14px; overflow-y: auto; scrollbar-width: none; }
.message-preview-rail::-webkit-scrollbar { display: none; }
.message-preview-mark { position: relative; flex: none; width: 14px; height: 10px; padding: 0; border: 0; background: transparent; cursor: pointer; }
.mark-line { position: absolute; top: 4px; right: 1px; width: 7px; height: 2px; border-radius: 1px; background: var(--peek-faint); transition: width 120ms ease, background 120ms ease; }
.message-preview-mark:hover .mark-line, .message-preview-mark.active .mark-line { width: 11px; background: var(--peek-accent); }
.message-preview-tooltip { position: fixed; z-index: 20; right: 30px; width: min(250px, calc(100vw - 48px)); padding: 6px 8px; border: 1px solid var(--peek-border); border-radius: 5px; background: var(--peek-list-bg); color: var(--peek-text); box-shadow: 0 6px 18px rgba(0, 0, 0, 0.24); font-size: 11px; line-height: 1.45; text-align: left; opacity: 0; visibility: hidden; pointer-events: none; transform: translateY(-4px); transition: opacity 100ms ease, transform 100ms ease; }
.message-preview-mark:hover .message-preview-tooltip { opacity: 1; visibility: visible; transform: translateY(0); }
.message-item.user { display: flex; justify-content: flex-end; width: 100%; }
.message-item.assistant { display: flex; justify-content: flex-start; width: 100%; }
.user-turn { display: flex; flex-direction: column; align-items: flex-end; gap: 6px; max-width: 78%; }
.user-images {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 8px;
  max-width: 100%;
  padding: 1px;
}
.user-image-btn {
  display: block;
  margin: 0;
  padding: 0;
  border: none;
  background: transparent;
  border-radius: 12px;
  overflow: hidden;
  cursor: zoom-in;
  max-width: min(280px, 72vw);
  line-height: 0;
  box-shadow: 0 0 0 1px var(--peek-border);
  transform: translateZ(0);
  transition: box-shadow 140ms ease;
}
.user-image-btn:hover {
  box-shadow: 0 0 0 1px color-mix(in srgb, var(--peek-accent) 55%, var(--peek-border));
}
.user-image {
  display: block;
  width: auto;
  height: auto;
  max-width: min(280px, 72vw);
  max-height: 360px;
  object-fit: contain;
  border-radius: inherit;
  user-select: none;
}
.user-bubble {
  width: fit-content;
  max-width: 100%;
  padding: 7px 11px;
  border: 1px solid color-mix(in srgb, var(--peek-user-bubble-border) 70%, transparent);
  border-radius: 14px 14px 5px 14px;
  background: var(--peek-user-bubble-bg);
  color: var(--peek-user-bubble-text);
  font-size: 13px;
  line-height: 1.5;
  white-space: pre-wrap;
  word-break: break-word;
  box-shadow: 0 1px 0 color-mix(in srgb, #000 4%, transparent);
}
.user-message-text { display: block; min-width: 0; }
.rewind-icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  padding: 0;
  border: 0;
  border-radius: 7px;
  background: transparent;
  color: var(--peek-muted);
  cursor: pointer;
  opacity: 0.7;
}
.rewind-icon-btn:hover:not(:disabled) {
  opacity: 1;
  color: var(--peek-accent);
  background: color-mix(in srgb, var(--peek-accent) 14%, transparent);
}
.rewind-icon-btn:disabled {
  cursor: default;
  opacity: 0.4;
}
.user-selection-quote { display: block; margin-top: 6px; color: color-mix(in srgb, var(--peek-user-bubble-text) 70%, var(--peek-muted)); font-size: 12px; line-height: 1.5; white-space: pre-wrap; word-break: break-word; }
.assistant-bubble {
  display: flex;
  flex-direction: column;
  gap: 6px;
  width: 100%;
  max-width: 94%;
  min-width: 0;
  padding: 0;
  color: var(--peek-text);
}
.assistant-bubble :deep(.markdown-body) {
  font-size: 13px;
  line-height: 1.6;
}
.assistant-bubble :deep(.agent-work),
.assistant-bubble :deep(.tool-activity-list),
.assistant-bubble :deep(.reasoning-block),
.assistant-bubble :deep(.ask-answer-card),
.assistant-bubble :deep(.image-analysis-card) {
  width: 100%;
  max-width: none;
  box-sizing: border-box;
}
</style>

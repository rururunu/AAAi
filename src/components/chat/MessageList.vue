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
          <div class="user-bubble">
            <span class="user-message-text">{{ userContent(message).message }}</span>
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
import Markdown from "@/components/chat/Markdown.vue";
import { AppConfirmDialog } from "@/components/ui/confirm-dialog";
import { rewindSession } from "@/services/ipc";
import { useSettingStore } from "@/stores/setting";
import type { ChatMessage, CheckpointInfo } from "@/types/chat";
import { parseSelectionAttachment } from "@/services/chat/selectionAttachment";
import { tr } from "@/services/i18n";

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
function activityLabel(message: ChatMessage) {
  if (!isPending(message) || message.askUserAnswer?.length) return "";

  const running = [...(message.toolActivities ?? [])]
    .reverse()
    .find((activity) => activity.status === "running");
  if (running) {
    if (running.toolName === "ask_user") return "";
    if (running.kind === "read") return tr(settingStore.language, "reading");
    if (["create", "edit", "delete", "move"].includes(running.kind)) {
      return tr(settingStore.language, "writing");
    }
    if (running.kind === "shell") return tr(settingStore.language, "runningCommand");
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
  () => props.messages.map((item) => `${item.id}:${item.content.length}:${item.reasoning?.length ?? 0}:${item.askUserAnswer?.map((a) => a.selected.join(",")).join(";") ?? ""}:${item.toolActivities?.map((activity) => `${activity.id}:${activity.status}:${activity.detail?.length ?? 0}`).join(",") ?? ""}:${item.status}`).join("|"),
  () => void scrollToBottomIfNeeded(),
  { immediate: true },
);
</script>

<style scoped>
.message-list-shell { position: relative; display: flex; flex: 1; min-height: 0; }
.message-list { flex: 1; min-height: 0; overflow-x: hidden; overflow-y: auto; overscroll-behavior: contain; padding: 12px 28px 8px 12px; display: flex; flex-direction: column; gap: 12px; scroll-padding-top: 12px; }
.message-preview-rail { position: absolute; z-index: 4; top: 42px; right: 7px; bottom: 10px; display: flex; flex-direction: column; gap: 5px; width: 14px; overflow-y: auto; scrollbar-width: none; }
.message-preview-rail::-webkit-scrollbar { display: none; }
.message-preview-mark { position: relative; flex: none; width: 14px; height: 10px; padding: 0; border: 0; background: transparent; cursor: pointer; }
.mark-line { position: absolute; top: 4px; right: 1px; width: 7px; height: 2px; border-radius: 1px; background: var(--peek-faint); transition: width 120ms ease, background 120ms ease; }
.message-preview-mark:hover .mark-line, .message-preview-mark.active .mark-line { width: 11px; background: var(--peek-accent); }
.message-preview-tooltip { position: fixed; z-index: 20; right: 30px; width: min(250px, calc(100vw - 48px)); padding: 6px 8px; border: 1px solid var(--peek-border); border-radius: 5px; background: var(--peek-list-bg); color: var(--peek-text); box-shadow: 0 6px 18px rgba(0, 0, 0, 0.24); font-size: 11px; line-height: 1.45; text-align: left; opacity: 0; visibility: hidden; pointer-events: none; transform: translateY(-4px); transition: opacity 100ms ease, transform 100ms ease; }
.message-preview-mark:hover .message-preview-tooltip { opacity: 1; visibility: visible; transform: translateY(0); }
.message-item.user { display: flex; justify-content: flex-end; width: 100%; }
.message-item.assistant { display: flex; justify-content: flex-start; width: 100%; }
.user-turn { display: flex; flex-direction: column; align-items: flex-end; gap: 4px; max-width: 82%; }
.user-bubble { width: fit-content; max-width: 100%; padding: 8px 12px; border: 1px solid var(--peek-user-bubble-border); border-radius: 12px 12px 4px 12px; background: var(--peek-user-bubble-bg); color: var(--peek-user-bubble-text); font-size: 13px; line-height: 1.5; white-space: pre-wrap; word-break: break-word; }
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
  width: 100%;
  max-width: 92%;
  min-width: 0;
  padding: 2px 0;
  color: var(--peek-text);
}
.assistant-bubble :deep(.agent-work),
.assistant-bubble :deep(.tool-activity-list),
.assistant-bubble :deep(.reasoning-block),
.assistant-bubble :deep(.ask-answer-card) {
  width: 100%;
  max-width: none;
  box-sizing: border-box;
}
</style>

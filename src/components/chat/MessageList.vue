<template>
  <div class="message-list-shell">
    <nav
      v-if="userMessages.length"
      class="message-preview-rail"
      :aria-label="tr(settingStore.language, 'userMessageNav')"
    >
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
      <div v-if="displayItems.length === 0" class="empty-thread">
        {{ emptyThreadPrompt }}
      </div>
      <article
        v-for="item in displayItems"
        :key="item.key"
        class="message-item"
        :class="item.kind"
        :data-message-id="item.message.id"
      >
        <div v-if="item.kind === 'user'" class="user-turn">
          <div
            v-if="userContent(item.message).images?.length"
            class="user-images"
            data-tauri-drag-region="false"
          >
            <button
              v-for="(img, idx) in userContent(item.message).images"
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
            v-if="userContent(item.message).attachedFiles?.length"
            class="user-attached-files"
            data-tauri-drag-region="false"
          >
            <div
              v-for="(file, idx) in userContent(item.message).attachedFiles"
              :key="`${file.path}-${idx}`"
              class="user-file-chip"
              :class="{ skipped: Boolean(file.skipped) }"
              :title="file.skipped ? `${file.path} (${file.skipped})` : file.path"
            >
              <img
                v-if="fileIconForPath(file.path)"
                class="user-file-icon-img"
                :src="fileIconForPath(file.path) || ''"
                alt=""
              />
              <File v-else :size="12" :stroke-width="1.75" aria-hidden="true" />
              <span class="user-file-name">{{ file.name }}</span>
            </div>
          </div>
          <div
            v-if="userContent(item.message).message || userContent(item.message).selection"
            class="user-bubble"
          >
            <span v-if="userContent(item.message).message" class="user-message-text">
              <template
                v-for="(part, partIdx) in inlineMessageParts(userContent(item.message).message)"
                :key="`${item.message.id}-part-${partIdx}`"
              >
                <span v-if="part.kind === 'mention'" class="user-mention-chip" :title="part.path">
                  <img
                    v-if="fileIconForPath(part.path)"
                    class="user-mention-icon"
                    :src="fileIconForPath(part.path) || ''"
                    alt=""
                  />
                  <File v-else :size="12" class="user-mention-fallback" />
                  <span class="user-mention-name">@{{ part.name }}</span>
                </span>
                <template v-else>{{ part.text }}</template>
              </template>
            </span>
            <span v-if="userContent(item.message).selection" class="user-selection-quote">
              {{ userContent(item.message).selection }}
            </span>
          </div>
          <div
            v-if="copyableUserText(item.message) || checkpointFor(item.message)"
            class="message-actions user-message-actions"
          >
            <button
              v-if="copyableUserText(item.message)"
              type="button"
              class="message-action-btn"
              :class="copyButtonClass(item.message.id)"
              :aria-label="copyButtonLabel(item.message.id)"
              :title="copyButtonLabel(item.message.id)"
              @click.stop="copyMessage(item.message, 'user')"
            >
              <Check
                v-if="copyStatus?.id === item.message.id && copyStatus.state === 'copied'"
                :size="14"
                :stroke-width="2"
                aria-hidden="true"
              />
              <Copy v-else :size="14" :stroke-width="2" aria-hidden="true" />
            </button>
            <button
              v-if="checkpointFor(item.message)"
              type="button"
              class="message-action-btn"
              :disabled="rewindBusy"
              :aria-label="tr(settingStore.language, 'rewind')"
              :title="tr(settingStore.language, 'rewind')"
              @click.stop="confirmRewind(item.message)"
            >
              <Undo2 :size="14" :stroke-width="2" aria-hidden="true" />
            </button>
          </div>
        </div>
        <div v-else class="assistant-bubble">
          <AgentWorkDetails
            :message="item.message"
            :language="settingStore.language"
            :show-reasoning="settingStore.showReasoning"
            :display-mode="settingStore.agentWorkDisplay"
            @inspect-subagent="emit('inspectSubagent', $event)"
          />
          <AskUserAnswerCard
            v-if="item.message.askUserAnswer?.length"
            :items="item.message.askUserAnswer"
          />
          <ImageAnalysisDetails
            v-for="(analysis, idx) in imageAnalysesForAssistant(item.message)"
            :key="`${item.message.id}-analysis-${idx}`"
            :model="analysis.model"
            :text="analysis.text"
          />
          <EnvironmentContextCard
            v-if="item.message.environmentContext"
            :context="item.message.environmentContext"
          />
          <div v-else-if="needsProviderSetup(item.message)" class="provider-setup-card">
            <p class="provider-setup-text">
              {{ providerSetupText(item.message) }}
            </p>
            <button type="button" class="provider-setup-btn" @click="openProviderSettings">
              {{ tr(settingStore.language, "configureProviderAction") }}
            </button>
          </div>
          <Markdown
            v-else-if="item.message.content"
            :content="item.message.content"
            @preview-image="emit('previewImage', $event)"
          />
          <div v-if="item.injects.length" class="soft-inject-list">
            <div
              v-for="inject in item.injects"
              :key="inject.id"
              class="soft-inject-chip"
              :data-message-id="inject.id"
            >
              <span class="soft-inject-label">{{ tr(settingStore.language, "softInjected") }}</span>
              <span class="soft-inject-text">{{ softInjectText(inject) }}</span>
            </div>
          </div>
          <AssistantActivityIndicator
            v-if="activityLabel(item.message)"
            :label="activityLabel(item.message)!"
          />
          <CodeChangesSummary
            v-if="item.message.status === 'done'"
            :message="item.message"
            :can-undo="Boolean(checkpointForAssistant(item.message))"
            :busy="rewindBusy"
            @undo="confirmAssistantRewind(item.message)"
            @review="$emit('reviewChanges')"
          />
          <div
            v-if="
              item.message.content.trim() ||
              processingDuration(item.message) ||
              turnTokenCount(item)
            "
            class="message-actions assistant-message-actions"
          >
            <span v-if="processingDuration(item.message)" class="processing-duration">
              {{
                tr(settingStore.language, "processedFor", {
                  duration: processingDuration(item.message)!,
                })
              }}
            </span>
            <span
              v-if="turnTokenCount(item)"
              class="token-usage"
              :title="tokenEstimateTitle(turnTokenCount(item))"
            >
              ≈ {{ formatTokenCount(turnTokenCount(item), settingStore.language) }} tokens
            </span>
            <button
              v-if="item.message.content.trim()"
              type="button"
              class="message-action-btn"
              :class="copyButtonClass(item.message.id)"
              :aria-label="copyButtonLabel(item.message.id)"
              :title="copyButtonLabel(item.message.id)"
              @click.stop="copyMessage(item.message, 'assistant')"
            >
              <Check
                v-if="copyStatus?.id === item.message.id && copyStatus.state === 'copied'"
                :size="14"
                :stroke-width="2"
                aria-hidden="true"
              />
              <Copy v-else :size="14" :stroke-width="2" aria-hidden="true" />
            </button>
          </div>
        </div>
      </article>
    </div>

    <AppConfirmDialog ref="confirmDialogRef" />
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { Check, Copy, File, Undo2 } from "@lucide/vue";
import { codeLanguageForPath } from "@/services/chat/codeLanguage";
import AgentWorkDetails from "@/components/chat/AgentWorkDetails.vue";
import CodeChangesSummary from "@/components/chat/CodeChangesSummary.vue";
import AssistantActivityIndicator from "@/components/chat/AssistantActivityIndicator.vue";
import AskUserAnswerCard from "@/components/chat/AskUserAnswerCard.vue";
import ImageAnalysisDetails from "@/components/chat/ImageAnalysisDetails.vue";
import EnvironmentContextCard from "@/components/chat/EnvironmentContextCard.vue";
import Markdown from "@/components/chat/Markdown.vue";
import { AppConfirmDialog } from "@/components/ui/confirm-dialog";
import { openSettings as ipcOpenSettings, rewindSession } from "@/services/ipc";
import { useSettingStore } from "@/stores/setting";
import type { ChatMessage, CheckpointInfo } from "@/types/chat";
import { parseSelectionAttachment } from "@/services/chat/selectionAttachment";
import { isSoftInjectContent, stripSoftInjectMarker } from "@/services/chat/softInject";
import { tr } from "@/services/i18n";
import { gsapScrollContainerTo } from "@/services/motion/gsapPresets";
import { copyText } from "@/services/clipboard";
import { estimateMessageTokens, formatTokenCount } from "@/services/chat/tokenEstimate";
import { isConfigureProviderError } from "@/services/chat/ensureDefaultModel";
import { useAppStore } from "@/stores/app";

type DisplayItem =
  | { kind: "user"; key: string; message: ChatMessage }
  | { kind: "assistant"; key: string; message: ChatMessage; injects: ChatMessage[] };
function previewImage(url: string) {
  emit("previewImage", url);
}

const SCROLL_NEAR_BOTTOM_THRESHOLD = 96;
const props = defineProps<{
  messages: ChatMessage[];
  sessionId?: string;
  workspaceName?: string;
  checkpoints?: CheckpointInfo[];
}>();
const workspaceName = computed(() => props.workspaceName?.trim() || "");
const emptyThreadPrompt = computed(() =>
  workspaceName.value
    ? tr(settingStore.language, "emptyWorkspaceThread", { workspace: workspaceName.value })
    : tr(settingStore.language, "emptyThreadGeneral"),
);
const emit = defineEmits<{
  rewound: [payload: { text: string }];
  reviewChanges: [];
  inspectSubagent: [activityId: string];
  previewImage: [source: string];
}>();
const settingStore = useSettingStore();
const appStore = useAppStore();
const confirmDialogRef = ref<InstanceType<typeof AppConfirmDialog> | null>(null);

function needsProviderSetup(message: ChatMessage): boolean {
  return message.status === "error" && isConfigureProviderError(message.content);
}

function providerSetupText(_message: ChatMessage): string {
  return tr(settingStore.language, "configureProviderHint");
}

function openProviderSettings() {
  appStore.openSettings("provider");
  void ipcOpenSettings().catch(() => {
    // Workbench may already be focused; app-store signal still opens settings.
  });
}
const visibleMessages = computed(() =>
  props.messages.filter((message) => {
    const role = String(message.role).toLowerCase();
    return role !== "system" && role !== "tool";
  }),
);

const displayItems = computed((): DisplayItem[] => {
  const items: DisplayItem[] = [];
  for (const message of visibleMessages.value) {
    if (isUserMessage(message)) {
      if (isSoftInjectMessage(message)) {
        let folded = false;
        for (let i = items.length - 1; i >= 0; i -= 1) {
          const item = items[i];
          if (item?.kind === "assistant") {
            item.injects.push(message);
            folded = true;
            break;
          }
        }
        // Mis-tagged first message (no prior assistant): show as a normal bubble.
        if (!folded) {
          items.push({ kind: "user", key: message.id, message });
        }
        continue;
      }
      items.push({ kind: "user", key: message.id, message });
      continue;
    }
    items.push({
      kind: "assistant",
      key: message.id,
      message,
      injects: [],
    });
  }
  return items;
});

const userMessages = computed(() =>
  displayItems.value
    .filter((item): item is Extract<DisplayItem, { kind: "user" }> => item.kind === "user")
    .map((item) => item.message),
);
const listRef = ref<HTMLElement | null>(null);
const stickToBottom = ref(true);
const activeUserMessageId = ref("");
const rewindBusy = ref(false);
const copyStatus = ref<{ id: string; state: "copied" | "failed" } | null>(null);
const durationClock = ref(Date.now());
let copyStatusTimer: number | undefined;
let durationTimer: number | undefined;

function normalizeRole(role: ChatMessage["role"] | string) {
  return String(role).toLowerCase();
}
function isUserMessage(message: ChatMessage) {
  return normalizeRole(message.role) === "user";
}
function isSoftInjectMessage(message: ChatMessage) {
  return message.injected === true || isSoftInjectContent(message.content);
}
function softInjectText(message: ChatMessage) {
  return parseSelectionAttachment(stripSoftInjectMarker(message.content)).message;
}
function userContent(message: ChatMessage) {
  return parseSelectionAttachment(stripSoftInjectMarker(message.content));
}

function fileIconForPath(path: string) {
  return codeLanguageForPath(path).icon;
}

type InlineMessagePart =
  { kind: "text"; text: string } | { kind: "mention"; path: string; name: string };

const MENTION_TOKEN_RE = /@(?:"([^"]+)"|([^\s@]+))/g;

function inlineMessageParts(text: string): InlineMessagePart[] {
  const parts: InlineMessagePart[] = [];
  let lastIndex = 0;
  const re = new RegExp(MENTION_TOKEN_RE.source, "g");
  let match: RegExpExecArray | null;
  while ((match = re.exec(text)) !== null) {
    if (match.index > lastIndex) {
      parts.push({ kind: "text", text: text.slice(lastIndex, match.index) });
    }
    const path = match[1] || match[2] || "";
    const name = path.split(/[/\\]/).pop() || path;
    parts.push({ kind: "mention", path, name });
    lastIndex = match.index + match[0].length;
  }
  if (lastIndex < text.length) {
    parts.push({ kind: "text", text: text.slice(lastIndex) });
  }
  return parts.length > 0 ? parts : [{ kind: "text", text }];
}

function copyableUserText(message: ChatMessage) {
  const content = userContent(message);
  return [content.message.trim(), content.selection?.trim() ?? ""].filter(Boolean).join("\n\n");
}

function copyButtonLabel(messageId: string) {
  if (copyStatus.value?.id !== messageId) return tr(settingStore.language, "copy");
  return tr(settingStore.language, copyStatus.value.state === "copied" ? "copied" : "copyFailed");
}

function copyButtonClass(messageId: string) {
  if (copyStatus.value?.id !== messageId) return undefined;
  return copyStatus.value.state;
}

async function copyMessage(message: ChatMessage, kind: "user" | "assistant") {
  const text = kind === "user" ? copyableUserText(message) : message.content;
  if (!text) return;
  if (copyStatusTimer) window.clearTimeout(copyStatusTimer);
  try {
    await copyText(text);
    copyStatus.value = { id: message.id, state: "copied" };
  } catch (error) {
    console.error("failed to copy message:", error);
    copyStatus.value = { id: message.id, state: "failed" };
  }
  copyStatusTimer = window.setTimeout(() => {
    if (copyStatus.value?.id === message.id) copyStatus.value = null;
    copyStatusTimer = undefined;
  }, 1600);
}

/** Image analyses are persisted on the preceding user message; show them on the assistant turn. */
function precedingUserMessage(assistant: ChatMessage): ChatMessage | undefined {
  const list = visibleMessages.value;
  const index = list.findIndex((item) => item.id === assistant.id);
  if (index <= 0) return undefined;
  for (let i = index - 1; i >= 0; i -= 1) {
    if (isUserMessage(list[i]!) && !isSoftInjectMessage(list[i]!)) {
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

function checkpointFor(message: ChatMessage) {
  return (props.checkpoints ?? []).find((item) => item.userMessageId === message.id);
}

function turnTokenCount(item: Extract<DisplayItem, { kind: "assistant" }>) {
  const user = precedingUserMessage(item.message);
  return [user, item.message, ...item.injects]
    .filter((message): message is ChatMessage => Boolean(message))
    .reduce((total, message) => total + estimateMessageTokens(message), 0);
}

function tokenEstimateTitle(tokens: number) {
  return tr(settingStore.language, "tokens.estimated", {
    count: new Intl.NumberFormat(settingStore.language).format(tokens),
  });
}

function processingDuration(message: ChatMessage): string | undefined {
  const startedAt = precedingUserMessage(message)?.timestamp;
  if (!startedAt) return undefined;

  const running = isPending(message);
  const finishedAt = running ? durationClock.value : message.completedAt;
  if (!finishedAt || finishedAt < startedAt) return undefined;

  const totalSeconds = Math.max(0, Math.floor((finishedAt - startedAt) / 1000));
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return minutes > 0 ? `${minutes} m ${seconds} s` : `${seconds} s`;
}
function checkpointForAssistant(message: ChatMessage) {
  const userMessage = precedingUserMessage(message);
  return userMessage ? checkpointFor(userMessage) : undefined;
}
function confirmAssistantRewind(message: ChatMessage) {
  const userMessage = precedingUserMessage(message);
  if (userMessage) void confirmRewind(userMessage);
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

function isWaitingForAskUser(message: ChatMessage) {
  return (message.toolActivities ?? []).some(
    (activity) => activity.toolName === "ask_user" && activity.status === "running",
  );
}

function activityLabel(message: ChatMessage) {
  if (message.activityStatus?.startsWith("stream_retry")) {
    const [, attemptRaw, maxRaw] = message.activityStatus.split(":");
    const attempt = Number.parseInt(attemptRaw ?? "1", 10) || 1;
    const max = Number.parseInt(maxRaw ?? "5", 10) || 5;
    return tr(settingStore.language, "streamRetrying", { attempt, max });
  }
  if (message.activityStatus === "reject_empty_completion") {
    return "检测到空完成，正在纠正并强制重试修改...";
  }
  if (!isPending(message) || isWaitingForAskUser(message)) return "";

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
    if (running.toolName === "ask_user") {
      return tr(settingStore.language, "waitingAnswer");
    }

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
  const padBottom = Number.parseFloat(getComputedStyle(element).paddingBottom) || 0;
  // Ignore composer clearance padding — it would otherwise make scrollTop=0
  // look "not at bottom" on short threads and break stick-to-bottom.
  const contentBottom = element.scrollHeight - padBottom;
  const viewportBottom = element.scrollTop + element.clientHeight;
  return contentBottom - viewportBottom <= SCROLL_NEAR_BOTTOM_THRESHOLD;
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
    const node = element.querySelector<HTMLElement>(
      `[data-message-id="${CSS.escape(message.id)}"]`,
    );
    if (node && node.getBoundingClientRect().top <= top) active = message.id;
  }
  activeUserMessageId.value = active;
}
function scrollToMessage(messageId: string) {
  const container = listRef.value;
  const node = container?.querySelector<HTMLElement>(
    `[data-message-id="${CSS.escape(messageId)}"]`,
  );
  if (!container || !node) return;
  stickToBottom.value = false;
  activeUserMessageId.value = messageId;
  // Scroll the message list scroller only — not the overlay / window.
  // Offset accounts for the absolute thread header overlay.
  gsapScrollContainerTo(container, node, { offsetY: 42 });
}
function messagePreview(message: ChatMessage) {
  const parsed = userContent(message);
  const compact = parsed.message.replace(/\s+/g, " ").trim();
  if (compact) {
    return compact.length > 72 ? `${compact.slice(0, 72)}...` : compact;
  }
  if (parsed.attachedFiles?.length) {
    return parsed.attachedFiles.map((file) => file.name).join(", ");
  }
  if (parsed.images?.length) {
    return parsed.images.length === 1 ? "image" : `${parsed.images.length} images`;
  }
  return "";
}
/**
 * Keep the latest user turn on-screen.
 *
 * Absolute scroll-to-bottom fights the large composer `padding-bottom`: on short
 * turns (especially while the overlay is still expanding) it scrolls the first
 * user bubble above the viewport. If the turn still fits, pin to the user
 * message (scrollTop 0 for the first turn); only follow the true bottom once
 * the reply no longer fits with that user message.
 */
async function scrollToBottomIfNeeded() {
  await nextTick();
  const element = listRef.value;
  if (!element) return;

  if (!stickToBottom.value) {
    updateActiveUserMessage();
    return;
  }

  const padBottom = Number.parseFloat(getComputedStyle(element).paddingBottom) || 0;
  const maxScroll = element.scrollHeight - element.clientHeight;
  if (maxScroll <= 1) {
    element.scrollTop = 0;
    updateActiveUserMessage();
    return;
  }

  const users = element.querySelectorAll<HTMLElement>(".message-item.user");
  const lastUser = users[users.length - 1];
  if (lastUser) {
    const listTop = element.getBoundingClientRect().top;
    const userTop = lastUser.getBoundingClientRect().top - listTop + element.scrollTop;
    const contentBottom = element.scrollHeight - padBottom;
    const turnHeight = contentBottom - userTop;

    // Whole turn still fits: keep the user bubble visible (top of thread for
    // the first message; otherwise align that user message near the top).
    if (turnHeight <= element.clientHeight - 4) {
      element.scrollTop = users.length <= 1 ? 0 : Math.max(0, userTop - 8);
      updateActiveUserMessage();
      return;
    }
  }

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
  () =>
    props.messages
      .map(
        (item) =>
          `${item.id}:${item.content.length}:${item.reasoning?.length ?? 0}:${item.askUserAnswer?.map((a) => a.selected.join(",")).join(";") ?? ""}:${item.toolActivities?.map((activity) => `${activity.id}:${activity.status}:${activity.detail?.length ?? 0}`).join(",") ?? ""}:${item.status}:${item.activityStatus ?? ""}`,
      )
      .join("|"),
  () => void scrollToBottomIfNeeded(),
  { immediate: true },
);

let resizeObserver: ResizeObserver | null = null;
onMounted(() => {
  durationTimer = window.setInterval(() => {
    if (visibleMessages.value.some(isPending)) durationClock.value = Date.now();
  }, 1000);
  const element = listRef.value;
  if (!element || typeof ResizeObserver === "undefined") return;
  resizeObserver = new ResizeObserver(() => {
    void scrollToBottomIfNeeded();
  });
  resizeObserver.observe(element);
});
onUnmounted(() => {
  resizeObserver?.disconnect();
  resizeObserver = null;
  if (copyStatusTimer) window.clearTimeout(copyStatusTimer);
  if (durationTimer) window.clearInterval(durationTimer);
});
</script>

<style scoped>
.message-list-shell {
  position: relative;
  display: flex;
  flex: 1;
  min-height: 0;
}
.message-list {
  flex: 1;
  min-height: 0;
  overflow-x: hidden;
  overflow-y: auto;
  overscroll-behavior: contain;
  padding: 12px 28px 10px 12px;
  display: flex;
  flex-direction: column;
  gap: 14px;
  scroll-padding-top: 12px;
}
.empty-thread {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 120px;
  color: var(--peek-muted);
  font-size: 13px;
  text-align: center;
  user-select: none;
}
.message-preview-rail {
  position: absolute;
  z-index: 4;
  top: 42px;
  right: 7px;
  bottom: 10px;
  display: flex;
  flex-direction: column;
  gap: 5px;
  width: 14px;
  overflow-y: auto;
  scrollbar-width: none;
}
.message-preview-rail::-webkit-scrollbar {
  display: none;
}
.message-preview-mark {
  position: relative;
  flex: none;
  width: 14px;
  height: 10px;
  padding: 0;
  border: 0;
  background: transparent;
  cursor: pointer;
}
.mark-line {
  position: absolute;
  top: 4px;
  right: 1px;
  width: 7px;
  height: 2px;
  border-radius: 1px;
  background: var(--peek-faint);
  transition:
    width 120ms ease,
    background 120ms ease;
}
.message-preview-mark:hover .mark-line,
.message-preview-mark.active .mark-line {
  width: 11px;
  background: var(--peek-accent);
}
.message-preview-tooltip {
  position: fixed;
  z-index: 20;
  right: 30px;
  width: min(250px, calc(100vw - 48px));
  padding: 6px 8px;
  border: 1px solid var(--peek-border);
  border-radius: 5px;
  background: var(--peek-list-bg);
  color: var(--peek-text);
  box-shadow: 0 6px 18px rgba(0, 0, 0, 0.24);
  font-size: 11px;
  line-height: 1.45;
  text-align: left;
  opacity: 0;
  visibility: hidden;
  pointer-events: none;
  transform: translateY(-4px);
  transition:
    opacity 100ms ease,
    transform 100ms ease;
}
.message-preview-mark:hover .message-preview-tooltip {
  opacity: 1;
  visibility: visible;
  transform: translateY(0);
}
.message-item.user {
  display: flex;
  justify-content: flex-end;
  width: 100%;
}
.message-item.assistant {
  display: flex;
  justify-content: flex-start;
  width: 100%;
}
.user-turn {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 6px;
  max-width: 78%;
}
.user-images {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 8px;
  max-width: 100%;
  padding: 1px;
}
.user-attached-files {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 6px;
  max-width: 100%;
}
.user-file-chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  max-width: min(220px, 100%);
  height: 26px;
  padding: 0 10px;
  border: 1px solid var(--peek-border);
  border-radius: 7px;
  background: color-mix(in srgb, var(--peek-user-bubble-bg) 88%, var(--peek-surface));
  color: var(--peek-user-bubble-text);
  font-size: 12px;
  font-weight: 500;
  line-height: 1;
}
.user-file-icon-img {
  flex: none;
  width: 13px;
  height: 13px;
  object-fit: contain;
}
.user-file-chip.skipped {
  opacity: 0.55;
}
.user-file-name {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
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
  padding: 9px 12px;
  border: 1px solid color-mix(in srgb, var(--peek-user-bubble-border) 70%, transparent);
  border-radius: 14px 14px 5px 14px;
  background: var(--peek-user-bubble-bg);
  color: var(--peek-user-bubble-text);
  font-size: 13px;
  line-height: 1.65;
  white-space: pre-wrap;
  word-break: break-word;
  overflow-wrap: anywhere;
  box-shadow: 0 1px 0 color-mix(in srgb, #000 4%, transparent);
}
.user-message-text {
  display: inline;
  min-width: 0;
}
.user-mention-chip {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  max-width: min(220px, 100%);
  height: 24px;
  margin: 0 4px 0 0;
  padding: 0 8px;
  border: 1px solid color-mix(in srgb, var(--peek-border) 88%, transparent);
  border-radius: 6px;
  background: color-mix(in srgb, var(--peek-input-bg) 78%, var(--peek-surface));
  color: var(--peek-text);
  font-size: 12px;
  font-weight: 550;
  line-height: 24px;
  vertical-align: middle;
  overflow: hidden;
}
.user-mention-icon {
  flex: none;
  width: 13px;
  height: 13px;
  object-fit: contain;
}
.user-mention-fallback {
  flex: none;
  color: var(--peek-muted);
}
.user-mention-name {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.message-actions {
  display: flex;
  align-items: center;
  gap: 2px;
  min-height: 26px;
}
.user-message-actions {
  justify-content: flex-end;
}
.assistant-message-actions {
  justify-content: flex-start;
}
.processing-duration,
.token-usage {
  margin-right: 4px;
  color: var(--peek-muted);
  font-size: 11px;
  font-variant-numeric: tabular-nums;
  opacity: 0.72;
}
.message-action-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  padding: 0;
  border: 0;
  border-radius: 7px;
  background: transparent;
  color: var(--peek-icon, var(--peek-muted));
  cursor: pointer;
  opacity: 0.88;
}
.message-action-btn:hover:not(:disabled) {
  opacity: 1;
  color: var(--peek-accent);
  background: color-mix(in srgb, var(--peek-accent) 14%, transparent);
}
.message-action-btn:focus-visible {
  outline: 2px solid color-mix(in srgb, var(--peek-accent) 55%, transparent);
  outline-offset: 1px;
}
.message-action-btn.copied {
  color: #36a269;
  opacity: 1;
}
.message-action-btn.failed {
  color: #d35f5f;
  opacity: 1;
}
.message-action-btn:disabled {
  cursor: default;
  opacity: 0.4;
}
.user-selection-quote {
  display: block;
  margin-top: 6px;
  color: color-mix(in srgb, var(--peek-user-bubble-text) 70%, var(--peek-muted));
  font-size: 12px;
  line-height: 1.5;
  white-space: pre-wrap;
  word-break: break-word;
}
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

.provider-setup-card {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 12px;
  max-width: 420px;
}

.provider-setup-text {
  margin: 0;
  font-size: 13px;
  line-height: 1.6;
  color: var(--peek-text);
  white-space: pre-wrap;
}

.provider-setup-btn {
  appearance: none;
  border: 1px solid color-mix(in srgb, var(--peek-text) 14%, transparent);
  background: var(--peek-text);
  color: var(--peek-bg, #fff);
  border-radius: 999px;
  padding: 7px 14px;
  font-size: 12.5px;
  font-weight: 560;
  cursor: pointer;
  transition:
    opacity 0.15s ease,
    transform 0.15s ease;
}

.provider-setup-btn:hover {
  opacity: 0.92;
}

.provider-setup-btn:active {
  transform: translateY(0.5px);
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

.soft-inject-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-top: 2px;
  max-width: 100%;
}

.soft-inject-chip {
  display: flex;
  flex-direction: column;
  gap: 2px;
  width: fit-content;
  max-width: 100%;
  padding: 6px 10px;
  border: 1px dashed color-mix(in srgb, var(--peek-accent) 35%, var(--peek-border));
  border-radius: 10px;
  background: color-mix(in srgb, var(--peek-accent) 8%, transparent);
  color: var(--peek-text);
}

.soft-inject-label {
  font-size: 10px;
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: color-mix(in srgb, var(--peek-accent) 75%, var(--peek-muted));
}

.soft-inject-text {
  font-size: 12px;
  line-height: 1.45;
  white-space: pre-wrap;
  word-break: break-word;
  color: color-mix(in srgb, var(--peek-text) 88%, var(--peek-muted));
}
</style>

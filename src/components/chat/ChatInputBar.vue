<template>
  <div class="chat-input-shell">
    <Transition :css="false" mode="out-in" @enter="gsapPickerEnter" @leave="gsapPickerLeave">
      <WorkspacePickerPanel
        v-if="workspacePickerOpen"
        key="workspace-picker"
        :title="tr(language, 'chatInput.workspacePanelTitle')"
        :quick-select-only="workspaceQuickSelectOnly"
        :workspaces="workspaces"
        :current-workspace="currentWorkspace"
        :selected-index="selectedIndex"
        :saving="workspaceSaving"
        :error="workspaceError"
        :new-workspace-label="tr(language, 'chatInput.newWorkspace')"
        :no-previous-workspaces-label="tr(language, 'chatInput.noPreviousWorkspaces')"
        @add-new="addWorkspaceFromFolder"
        @select="chooseWorkspace"
      />

      <AskUserPicker
        v-else-if="showAskUserPicker"
        key="ask-user-list"
        :header="activeAskQuestion?.header"
        :question="activeAskQuestion?.question"
        :question-index="askQuestionIndex"
        :question-count="askQuestionCount"
        :options="activeAskOptions"
        :multi-select="activeAskQuestion?.multiSelect"
        :confirm-row-index="askConfirmRowIndex"
        :confirm-label="tr(language, 'confirmSelection')"
        :selected-index="selectedIndex"
        :ariaLabel="tr(language, 'select')"
        :is-option-selected="isAskOptionSelected"
        @hover="selectedIndex = $event"
        @select="selectAskOption"
        @confirm="confirmAskSelection"
      />

      <PathPermissionPicker
        v-else-if="showPathPermissionPicker"
        key="path-permission-list"
        :header="pathPermissionHeader"
        :question="pathPermissionQuestion"
        :path="props.pathPermission?.path"
        :options="pathPermissionOptions"
        :selected-index="selectedIndex"
        :ariaLabel="tr(language, 'permissionRequest')"
        @hover="selectedIndex = $event"
        @select="selectPathPermission"
      />

      <ToolApprovalPicker
        v-else-if="showToolApprovalPicker"
        key="tool-approval-list"
        :header="toolApprovalHeader"
        :options="toolApprovalOptions"
        :selected-index="selectedIndex"
        :ariaLabel="tr(language, 'toolApprovalTitle')"
        @hover="selectedIndex = $event"
        @select="selectToolApproval"
      />

      <HistoryPicker
        v-else-if="showHistoryPicker"
        key="history-list"
        :items="historyItems"
        :selected-index="selectedIndex"
        :empty-text="historyEmptyText"
        :ariaLabel="tr(language, 'chatHistory')"
        :format-time="formatTime"
        @hover="selectedIndex = $event"
        @select="selectHistorySession"
      />

      <ModelPicker
        v-else-if="showModelPicker"
        key="model-list"
        :models="modelPickerModels"
        :selected-model-id="chatModel"
        :selected-provider="chatModelProvider"
        :selected-index="selectedIndex"
        :loading="chatModelStore.loading"
        :refreshing="chatModelStore.refreshing"
        :error="chatModelStore.error"
        :loading-text="modelStatusText.loading"
        :empty-text="modelPickerEmptyText"
        :refresh-text="tr(language, 'refreshModels')"
        :ariaLabel="tr(language, 'chooseModel')"
        @hover="selectedIndex = $event"
        @select="selectModel"
        @refresh="refreshModelList"
      />

      <OptionPicker
        v-else-if="showChatModePicker"
        key="chat-mode-list"
        :options="chatModePickerOptions"
        :selected-id="settingStore.chatMode"
        :selected-index="selectedIndex"
        :ariaLabel="tr(language, 'chooseChatMode')"
        @hover="selectedIndex = $event"
        @select="selectChatMode"
      />

      <OptionPicker
        v-else-if="showThinkingTierList"
        key="thinking-tier-list"
        :options="thinkingTierPickerOptions"
        :selected-id="chatModel"
        :selected-index="selectedIndex"
        :ariaLabel="tr(language, 'chooseThinkingTier')"
        @hover="selectedIndex = $event"
        @select="selectThinkingTier"
      />

      <OptionPicker
        v-else-if="showApprovalPicker"
        key="approval-mode-list"
        :options="approvalPickerOptions"
        :selected-id="settingStore.toolApprovalMode"
        :selected-index="selectedIndex"
        :ariaLabel="tr(language, 'toolApprovalMode')"
        @hover="selectedIndex = $event"
        @select="selectApprovalMode"
      />

      <FileMentionPicker
        v-else-if="showFileSuggestions"
        key="file-suggestions"
        :loading="workspaceFilesLoading"
        :suggestions="fileSuggestions"
        :selected-index="selectedIndex"
        :loading-text="tr(language, 'loadingFiles')"
        :empty-text="tr(language, 'noMatchingFiles')"
        :ariaLabel="tr(language, 'workspace')"
        @hover="selectedIndex = $event"
        @select="selectWorkspaceFile"
      />

      <CommandSuggestions
        v-else-if="showCommandSuggestions"
        key="command-list"
        :commands="filteredCommands"
        :selected-index="selectedIndex"
        :ariaLabel="tr(language, 'commandSuggestions')"
        @hover="selectedIndex = $event"
        @select="executeCommand"
      />
    </Transition>

    <div
      class="input-bar"
      :class="{
        'has-images': attachedImages.length > 0 || attachedFiles.length > 0,
        'drag-over': fileDragOver,
      }"
      @dragover.prevent="onFileDragOver"
      @dragleave="onFileDragLeave"
      @drop.prevent="onFileDrop"
    >
      <div v-if="attachedImages.length" class="input-images peek-scrollbar" data-tauri-drag-region="false">
        <div
          v-for="(img, idx) in attachedImages"
          :key="idx"
          class="image-thumb-container"
          data-tauri-drag-region="false"
        >
          <img
            :src="img"
            class="image-thumb"
            draggable="false"
            data-no-drag
            @mousedown.stop
            @click.stop="previewImage(img)"
          />
          <button
            type="button"
            class="image-remove-btn"
            title="Remove image"
            @click="removeAttachedImage(idx)"
          >
            <X :size="10" />
          </button>
        </div>
      </div>

      <div v-if="attachedFiles.length" class="input-files peek-scrollbar" data-tauri-drag-region="false">
        <div
          v-for="(file, idx) in attachedFiles"
          :key="`${file.path}-${idx}`"
          class="file-chip"
          :class="{ skipped: Boolean(file.skippedReason) }"
          data-tauri-drag-region="false"
          :title="file.skippedReason ? `${file.path} (${file.skippedReason})` : file.path"
        >
          <File :size="12" :stroke-width="1.75" class="file-chip-icon" aria-hidden="true" />
          <span class="file-chip-name">{{ file.name }}</span>
          <button
            type="button"
            class="file-chip-remove"
            :aria-label="tr(language, 'close')"
            @click.stop="removeAttachedFile(idx)"
          >
            <X :size="11" :stroke-width="2" />
          </button>
        </div>
      </div>

      <div class="input-content">
      <span
        v-if="prefixText"
        class="input-prefix"
        data-tauri-drag-region="false"
        @click="focusInput"
      >{{ prefixText }}</span>
      <span
        v-if="props.selectionLines"
        class="selection-tag"
        data-tauri-drag-region="false"
        :title="`Selected ${props.selectionLines} lines`"
      >
        <span>select-{{ props.selectionLines }}</span>
      </span>
      <span
        v-if="pastedLineCount"
        class="selection-tag text-tag"
        data-tauri-drag-region="false"
        :title="`Pasted ${pastedLineCount} lines`"
      >
        <span>text-{{ pastedLineCount }}</span>
      </span>
      <span v-if="mentionedFiles.length" class="file-mention-tags peek-scrollbar">
        <span
          v-for="path in mentionedFiles"
          :key="path"
          class="selection-tag file-mention-tag"
          data-tauri-drag-region="false"
          :title="path"
        >
          <File :size="11" />
          <span class="file-mention-name">@{{ fileName(path) }}</span>
        </span>
      </span>
      <input
        ref="inputRef"
        v-model="message"
        type="text"
        :placeholder="inputPlaceholder"
        class="chat-input"
        data-tauri-drag-region="false"
        spellcheck="false"
        autocomplete="off"
        role="combobox"
        aria-autocomplete="list"
        :aria-expanded="showSuggestions || interactivePickerOpen"
        :readonly="inputLockedForTyping"
        @keydown="handleKeydown"
        @paste="handlePaste"
      />
      </div>

      <div class="input-footer">
        <div class="input-footer-primary">
      <div
        v-if="props.showWorkspaceButton"
        class="workspace-control"
        :class="{ active: Boolean(currentWorkspace), open: workspacePickerOpen }"
      >
        <button
          type="button"
          class="workspace-btn"
          data-tauri-drag-region="false"
          :title="workspaceTooltip"
          @click.stop="toggleWorkspacePicker"
        >
          <Folder :size="14" />
          <span v-if="currentWorkspace" class="workspace-name">{{ currentWorkspace.name }}</span>
          <span v-else class="workspace-name">{{ tr(language, "workspace") }}</span>
        </button>
        <button
          v-if="currentWorkspace"
          type="button"
          class="workspace-exit-btn"
          data-tauri-drag-region="false"
          :title="tr(language, 'chatInput.exitWorkspace')"
          :aria-label="tr(language, 'chatInput.exitWorkspace')"
          @click.stop="exitCurrentWorkspace"
        >
          <X :size="13" />
        </button>
      </div>

      <div class="model-picker">
        <button
          type="button"
          class="model-badge footer-chip"
          data-tauri-drag-region="false"
          :class="{ open: chatModePickerOpen }"
          :title="chatModeBadgeTitle"
          :aria-label="chatModeBadgeTitle"
          aria-haspopup="listbox"
          :aria-expanded="chatModePickerOpen"
          @mousedown.stop
          @click.stop="toggleChatModeMenu"
        >
          <component
            :is="chatModeIcon"
            :size="13"
            class="footer-chip-icon"
          />
          <span class="model-name">{{ chatModeLabel }}</span>
          <ChevronDown :size="11" class="model-chevron" />
        </button>
      </div>

      <div class="model-picker">
        <button
          type="button"
          class="model-badge footer-chip"
          data-tauri-drag-region="false"
          :class="{ open: modelPickerOpen, confirm: modelChipConfirm }"
          :title="modelBadgeTitle"
          :aria-label="modelBadgeTitle"
          aria-haspopup="listbox"
          :aria-expanded="modelPickerOpen"
          @mousedown.stop
          @click.stop="toggleModelMenu"
        >
          <span class="footer-chip-icon-slot" aria-hidden="true">
            <component
              :is="currentModelProviderIcon"
              v-if="currentModelProviderIcon"
              :size="13"
              class="footer-chip-icon"
            />
          </span>
          <span class="model-name" :key="currentModelDisplayName">{{ currentModelDisplayName }}</span>
          <ChevronDown :size="11" class="model-chevron" />
        </button>
      </div>

      <div
        class="model-picker thinking-tier-slot"
        :class="{ dormant: !showThinkingTierPicker }"
        :aria-hidden="!showThinkingTierPicker"
      >
        <button
          type="button"
          class="model-badge footer-chip"
          data-tauri-drag-region="false"
          :class="{ open: thinkingTierPickerOpen }"
          :title="thinkingTierBadgeTitle"
          :aria-label="thinkingTierBadgeTitle"
          aria-haspopup="listbox"
          :aria-expanded="thinkingTierPickerOpen"
          :tabindex="showThinkingTierPicker ? 0 : -1"
          :disabled="!showThinkingTierPicker"
          @mousedown.stop
          @click.stop="toggleThinkingTierMenu"
        >
          <Brain :size="13" class="footer-chip-icon" />
          <span class="model-name">{{ currentThinkingTierLabel || "—" }}</span>
          <ChevronDown :size="11" class="model-chevron" />
        </button>
      </div>

      <div
        class="model-picker approval-slot"
        :class="{ dormant: settingStore.chatMode === 'ask' }"
        :aria-hidden="settingStore.chatMode === 'ask'"
      >
        <button
          type="button"
          class="model-badge footer-chip"
          data-tauri-drag-region="false"
          :class="{ open: approvalPickerOpen }"
          :title="approvalBadgeTitle"
          :aria-label="approvalBadgeTitle"
          aria-haspopup="listbox"
          :aria-expanded="approvalPickerOpen"
          :tabindex="settingStore.chatMode === 'ask' ? -1 : 0"
          :disabled="settingStore.chatMode === 'ask'"
          @mousedown.stop
          @click.stop="toggleApprovalMenu"
        >
          <component
            :is="getApprovalIcon(settingStore.toolApprovalMode)"
            :size="13"
            class="footer-chip-icon"
          />
          <span class="model-name">{{ approvalModeLabel }}</span>
          <ChevronDown :size="11" class="model-chevron" />
        </button>
      </div>

        </div>

        <div class="input-footer-actions">
          <slot name="actions" />

      <ContextUsageRing
        :ratio="contextUsage.usageRatio"
        :tooltip="contextUsageTooltip"
      />

      <button
        v-if="sending && canSend"
        type="button"
        class="send-btn pause"
        data-tauri-drag-region="false"
        :aria-label="tr(language, 'pause')"
        :disabled="interactivePickerOpen"
        @click="emit('pause')"
      >
        <svg viewBox="0 0 16 16" fill="none" aria-hidden="true">
          <path
            d="M5.25 4.5V11.5M10.75 4.5V11.5"
            stroke="currentColor"
            stroke-width="1.6"
            stroke-linecap="round"
          />
        </svg>
      </button>

      <button
        type="button"
        class="send-btn"
        data-tauri-drag-region="false"
        :class="showPauseIcon ? 'pause' : canSend ? 'active' : ''"
        :aria-label="tr(language, showPauseIcon ? 'pause' : 'send')"
        :title="sending && canSend ? tr(language, 'attachInjectHint') : undefined"
        :disabled="interactivePickerOpen"
        @click="submit"
      >
        <svg v-if="!showPauseIcon" viewBox="0 0 16 16" fill="none" aria-hidden="true">
          <path
            d="M8 2.25L9.35 6.15L13.25 7.5L9.35 8.85L8 12.75L6.65 8.85L2.75 7.5L6.65 6.15L8 2.25Z"
            stroke="currentColor"
            stroke-width="1.35"
            stroke-linejoin="round"
          />
        </svg>
        <svg v-else viewBox="0 0 16 16" fill="none" aria-hidden="true">
          <path
            d="M5.25 4.5V11.5M10.75 4.5V11.5"
            stroke="currentColor"
            stroke-width="1.6"
            stroke-linecap="round"
          />
        </svg>
      </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { useDebounceFn, useEventListener } from "@vueuse/core";
import { storeToRefs } from "pinia";
import { gsapPickerEnter, gsapPickerLeave } from "@/services/motion/gsapPresets";
import { ChevronDown, File, Folder, X, Zap, Bot, MessageCircle, Brain, ShieldQuestion, Shield, Unlock } from "@lucide/vue";
import HistoryPicker from "./input/HistoryPicker.vue";
import ModelPicker from "./input/ModelPicker.vue";
import OptionPicker from "./input/OptionPicker.vue";
import AskUserPicker from "./input/AskUserPicker.vue";
import PathPermissionPicker from "./input/PathPermissionPicker.vue";
import ToolApprovalPicker from "./input/ToolApprovalPicker.vue";
import FileMentionPicker from "./input/FileMentionPicker.vue";
import CommandSuggestions from "./input/CommandSuggestions.vue";
import WorkspacePickerPanel from "./input/WorkspacePickerPanel.vue";
import ContextUsageRing from "./ContextUsageRing.vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { executeSlashCommand, fetchEnvironmentContext, slashCommands } from "@/commands/slash";
import { getContextUsage, setOverlayPopupOpen } from "@/services/ipc";
import { tr } from "@/services/i18n";
import {
  formatAttachedFilesForMessage,
  isImageFile,
  readAttachedFile,
  type AttachedFileChip,
} from "@/services/chat/attachFiles";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useChatModelStore } from "@/stores/chatModel";
import { useSettingStore } from "@/stores/setting";
import {
  getProviderIcon,
  getModelDisplayLabel,
  getModelDisplaySubtitle,
  groupModelsByProvider,
} from "@/lib/providerIcons";
import {
  findModelEntry,
  getActiveThinkingVariant,
  getThinkingTierOptions,
  isKnownModelSelection,
  isModelEntrySelected,
  localizeThinkingTierLabel,
  modelHasThinkingVariants,
} from "@/lib/modelThinking";
import { formatTokenCount } from "@/lib/formatTokens";
import { useChatStore } from "@/stores/chat";
import {
  localizedOptionLabel,
  toolApprovalModeOptions,
  type ChatMode,
  type ToolApprovalMode,
} from "@/types/setting";
import type {
  AskDisplayOption,
  AskUserQuestion,
  CapturedContext,
  ChatModelInfo,
  ContextUsageSnapshot,
  ChatSessionSummary,
  PathPermissionDecision,
  ToolApprovalDecision,
  ToolApprovalSession,
} from "@/types/chat";
import {
  clearCurrentWorkspace,
  createWorkspace,
  getCurrentWorkspace,
  listWorkspaces,
  listWorkspaceFiles,
  selectWorkspaceFolder,
  switchWorkspace,
  type Workspace,
} from "@/commands/workspace";
import { compressImageDataUrl } from "@/services/chat/compressImage";

export interface AskUserSession {
  requestId: string;
  questions: AskUserQuestion[];
}

export interface PathPermissionSession {
  requestId: string;
  path: string;
  operation: string;
  toolName: string;
}

const ASK_SKIP_MARKER = "__user_supplement__";

const props = withDefaults(
  defineProps<{
    sending?: boolean;
    placeholder?: string;
    enableCommands?: boolean;
    closeOnEscape?: boolean;
    askUser?: AskUserSession | null;
    pathPermission?: PathPermissionSession | null;
    toolApproval?: ToolApprovalSession | null;
    historySessions?: ChatSessionSummary[] | null;
    showWorkspaceButton?: boolean;
    selectionLines?: number;
    sessionId?: string;
    capturedContext?: CapturedContext | null;
    contextReady?: boolean;
  }>(),
  {
    sending: false,
    placeholder: "",
    enableCommands: true,
    closeOnEscape: true,
    showWorkspaceButton: false,
    selectionLines: 0,
    sessionId: "",
    capturedContext: null,
    contextReady: false,
  },
);

const emit = defineEmits<{
  submit: [message: string];
  pause: [];
  close: [];
  askUserComplete: [answer: string];
  pathPermissionComplete: [decision: PathPermissionDecision];
  toolApprovalComplete: [decision: ToolApprovalDecision];
  openHistory: [];
  historySelect: [sessionId: string];
  historyClose: [];
  removeSelection: [];
  showContext: [context: CapturedContext];
  previewImage: [source: string];
  layoutChange: [
    payload: {
      showSuggestions: boolean;
      suggestionCount: number;
      showModelMenu: boolean;
      modelMenuHeight: number;
      askUserRowCount: number;
      pickerRowCount: number;
      pickerHeight?: number;
      hasImages?: boolean;
      hasFiles?: boolean;
      isPreviewOpen?: boolean;
    },
  ];
  modelChange: [modelId: string];
}>();

const message = ref("");
const prefixText = ref("");
const pastedText = ref("");
const mentionedFiles = ref<string[]>([]);
const attachedImages = ref<string[]>([]);
const attachedFiles = ref<AttachedFileChip[]>([]);
const fileDragOver = ref(false);

function previewImage(url: string) {
  emit("previewImage", url);
}

function removeAttachedImage(index: number) {
  attachedImages.value.splice(index, 1);
  emitLayoutChange();
}

function removeAttachedFile(index: number) {
  attachedFiles.value.splice(index, 1);
  collapsePrefixIfNeeded();
  emitLayoutChange();
}

async function ingestDroppedOrPastedFiles(files: FileList | File[]) {
  const list = Array.from(files);
  if (list.length === 0) return;
  lockPrefixFromMessage();
  for (const file of list) {
    if (isImageFile(file)) {
      const dataUrl = await new Promise<string | null>((resolve) => {
        const reader = new FileReader();
        reader.onload = () => resolve(String(reader.result ?? "") || null);
        reader.onerror = () => resolve(null);
        reader.readAsDataURL(file);
      });
      if (!dataUrl) continue;
      const compressed = await compressImageDataUrl(dataUrl);
      attachedImages.value.push(compressed);
      continue;
    }
    const chip = await readAttachedFile(file);
    attachedFiles.value.push(chip);
  }
  emitLayoutChange();
}

function onFileDragOver(event: DragEvent) {
  if (!event.dataTransfer?.types?.includes("Files")) return;
  fileDragOver.value = true;
}

function onFileDragLeave() {
  fileDragOver.value = false;
}

async function onFileDrop(event: DragEvent) {
  fileDragOver.value = false;
  const files = event.dataTransfer?.files;
  if (!files?.length) return;
  await ingestDroppedOrPastedFiles(files);
}

async function applyCapturedImages(images?: string[]) {
  if (!images?.length) {
    return;
  }
  const compressed = await Promise.all(
    images.map((url) => compressImageDataUrl(url)),
  );
  attachedImages.value = compressed;
  emitLayoutChange();
}

const inputRef = ref<HTMLInputElement | null>(null);
const selectedIndex = ref(0);

watch(selectedIndex, async () => {
  await nextTick();
  const activeEl = document.querySelector(".command-item.active");
  if (activeEl) {
    activeEl.scrollIntoView({
      behavior: "auto",
      block: "nearest",
    });
  }
});

const modelPickerOpen = ref(false);
const modelChipConfirm = ref(false);
let modelChipConfirmTimer: ReturnType<typeof setTimeout> | null = null;
const approvalPickerOpen = ref(false);
const thinkingTierPickerOpen = ref(false);
const chatModePickerOpen = ref(false);
const askQuestionIndex = ref(0);
const askAnswers = ref<Record<number, string[]>>({});
const askUserFinishing = ref(false);

const settingStore = useSettingStore();
const chatStore = useChatStore();
const chatModelStore = useChatModelStore();
const { language } = storeToRefs(settingStore);
const { sessions } = storeToRefs(chatStore);

const contextUsage = ref<ContextUsageSnapshot>({
  usageRatio: 0,
  estimatedTokens: 0,
  contextWindowTokens: settingStore.largeContextEnabled ? 1_000_000 : 64_000,
});

function buildDraftMessage() {
  const parts = [prefixText.value, message.value].filter(Boolean);
  return parts.join(" ").trim();
}

const refreshContextUsage = useDebounceFn(async () => {
  try {
    const response = await getContextUsage({
      sessionId: props.sessionId || undefined,
      draftMessage: buildDraftMessage() || undefined,
      context: props.capturedContext ?? undefined,
    });
    contextUsage.value = {
      usageRatio: response.usageRatio,
      estimatedTokens: response.estimatedTokens,
      contextWindowTokens: response.contextWindowTokens,
    };
    if (props.sessionId) {
      chatStore.setContextUsage(props.sessionId, contextUsage.value);
    }
  } catch (error) {
    console.error("Failed to load context usage:", error);
  }
}, 180);

const contextUsageTooltip = computed(() =>
  tr(language.value, "contextUsageHint", {
    used: formatTokenCount(contextUsage.value.estimatedTokens),
    total: formatTokenCount(contextUsage.value.contextWindowTokens),
  }),
);

function sessionMessagesFingerprint(sessionId: string) {
  const messages = sessions.value[sessionId] ?? [];
  let chars = 0;
  for (const item of messages) {
    chars += item.content.length + (item.reasoning?.length ?? 0);
  }
  return `${messages.length}:${chars}`;
}

watch(
  () => [
    props.sessionId,
    props.capturedContext,
    settingStore.largeContextEnabled,
    props.sessionId ? sessionMessagesFingerprint(props.sessionId) : "",
  ] as const,
  () => {
    void refreshContextUsage();
  },
);

watch([message, prefixText], () => {
  void refreshContextUsage();
});

watch(
  () => props.capturedContext?.selectedImages,
  (images) => {
    void applyCapturedImages(images);
  },
  { immediate: true, deep: true },
);

function formatTime(timestamp: number) {
  const date = new Date(timestamp);
  const now = new Date();
  
  const isToday = date.toDateString() === now.toDateString();
  if (isToday) {
    return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", hour12: false });
  }
  
  const yesterday = new Date(now);
  yesterday.setDate(now.getDate() - 1);
  const isYesterday = date.toDateString() === yesterday.toDateString();
  if (isYesterday) {
    return tr(language.value, "yesterday");
  }
  
  return `${date.getMonth() + 1}/${date.getDate()}`;
}

// Bound to global settings used by backend provider resolution.
const chatModel = ref(settingStore.chatModel);
const chatModelProvider = ref(settingStore.chatModelProvider);

watch(
  () => settingStore.chatModel,
  (next) => {
    if (next !== chatModel.value) {
      chatModel.value = next;
    }
  },
);

watch(
  () => settingStore.chatModelProvider,
  (next) => {
    if (next !== chatModelProvider.value) {
      chatModelProvider.value = next;
    }
  },
);

const modelStatusText = computed(() => ({
  loading: tr(language.value, "loadingModels"),
  empty: tr(language.value, "noModels"),
}));

const availableModels = computed(() => {
  const models = [...chatModelStore.models];
  const current = chatModel.value.trim();

  if (
    current &&
    models.length > 0 &&
    !models.some((model) =>
      isModelEntrySelected(model, current, chatModelProvider.value),
    )
  ) {
    models.unshift({ id: current, ownedBy: "", provider: chatModelProvider.value });
  }

  return models;
});

/** Draft stashed while the model list uses the input as a filter query. */
const modelPickerDraft = ref<string | null>(null);

function beginModelFilterSession() {
  if (modelPickerDraft.value !== null) {
    return;
  }
  modelPickerDraft.value = message.value;
  message.value = "";
}

function endModelFilterSession() {
  if (modelPickerDraft.value === null) {
    return;
  }
  message.value = modelPickerDraft.value;
  modelPickerDraft.value = null;
}

function modelMatchesFilter(model: (typeof availableModels.value)[number], query: string) {
  const q = query.trim().toLowerCase();
  if (!q) {
    return true;
  }
  const haystack = [
    model.id,
    model.provider,
    model.ownedBy,
    getModelDisplayLabel(model),
    getModelDisplaySubtitle(model) ?? "",
  ]
    .join(" ")
    .toLowerCase();
  return haystack.includes(q);
}

const modelPickerModels = computed(() => {
  const models = availableModels.value;
  if (!modelPickerOpen.value) {
    return models;
  }
  return models.filter((model) => modelMatchesFilter(model, message.value));
});

const modelPickerEmptyText = computed(() => {
  if (modelPickerOpen.value && message.value.trim()) {
    return tr(language.value, "noMatchingModels");
  }
  return modelStatusText.value.empty;
});

const currentModelEntry = computed(() =>
  findModelEntry(availableModels.value, chatModel.value, chatModelProvider.value),
);

const showThinkingTierPicker = computed(() => {
  const entry = currentModelEntry.value;
  return entry ? modelHasThinkingVariants(entry) : false;
});

function thinkingTierIcon(label: string) {
  switch (label.trim().toLowerCase()) {
    case "low":
      return Zap;
    case "high":
      return Brain;
    case "agent":
      return Bot;
    case "default":
      return MessageCircle;
    default:
      return Brain;
  }
}

const thinkingTierPickerOptions = computed(() => {
  const entry = currentModelEntry.value;
  if (!entry) {
    return [];
  }
  return getThinkingTierOptions(entry).map((variant) => {
    const label = localizeThinkingTierLabel(variant.label, language.value);
    return {
      id: variant.id,
      label,
      icon: thinkingTierIcon(variant.label),
    };
  });
});

const currentThinkingTierLabel = computed(() => {
  const entry = currentModelEntry.value;
  if (!entry) {
    return "";
  }
  const active = getActiveThinkingVariant(entry, chatModel.value);
  return active ? localizeThinkingTierLabel(active.label, language.value) : "";
});

const thinkingTierBadgeTitle = computed(() =>
  tr(language.value, "currentThinkingTier", { tier: currentThinkingTierLabel.value }),
);

const currentModelProviderIcon = computed(() => {
  return getProviderIcon(currentModelEntry.value?.provider);
});

const currentModelDisplayName = computed(() => {
  const current = chatModel.value.trim();
  if (!current || (chatModelStore.models.length === 0 && !chatModelStore.loading)) {
    return tr(language.value, "chooseModel");
  }
  const match = currentModelEntry.value;
  if (!match && chatModelStore.models.length === 0) {
    return tr(language.value, "chooseModel");
  }
  return getModelDisplayLabel(
    match ?? { id: current, provider: "", displayName: undefined },
  );
});

const modelBadgeTitle = computed(() => {
  const current = chatModel.value.trim();
  if (!current || chatModelStore.models.length === 0) {
    return tr(language.value, "chooseModel");
  }
  const match = currentModelEntry.value;
  return tr(language.value, "currentModel", {
    model: getModelDisplayLabel(
      match ?? { id: current, provider: "", displayName: undefined },
    ),
  });
});
const chatModeLabel = computed(() =>
  settingStore.chatMode === "ask"
    ? tr(language.value, "chatModeAsk")
    : tr(language.value, "chatModeAgent"),
);
const chatModeBadgeTitle = computed(() =>
  settingStore.chatMode === "ask"
    ? tr(language.value, "currentChatModeAsk")
    : tr(language.value, "currentChatModeAgent"),
);
const chatModeIcon = computed(() =>
  settingStore.chatMode === "ask" ? MessageCircle : Bot,
);

function getApprovalIcon(mode: ToolApprovalMode) {
  switch (mode) {
    case "ask":
      // Ask before each tool — shield with question.
      return ShieldQuestion;
    case "auto":
      // Auto-run under policy — guarded shield.
      return Shield;
    case "alwaysAllow":
      // No prompts (dangerous shell still blocked) — unlocked.
      return Unlock;
  }
}

const chatModePickerOptions = computed(() => [
  {
    id: "ask",
    label: tr(language.value, "chatModeAsk"),
    description: tr(language.value, "chatModeAskDesc"),
    icon: MessageCircle,
  },
  {
    id: "agent",
    label: tr(language.value, "chatModeAgent"),
    description: tr(language.value, "chatModeAgentDesc"),
    icon: Bot,
  },
]);
const approvalPickerOptions = computed(() =>
  toolApprovalModeOptions.map((option) => ({
    id: option.value,
    label: localizedOptionLabel(option, language.value),
    icon: getApprovalIcon(option.value),
  })),
);
const approvalModeLabel = computed(() => {
  const current = approvalPickerOptions.value.find(
    (option) => option.id === settingStore.toolApprovalMode,
  );
  return current?.label ?? tr(language.value, "toolApprovalMode");
});
const approvalBadgeTitle = computed(() =>
  tr(language.value, "currentApprovalMode", { mode: approvalModeLabel.value }),
);

const showHistoryPicker = computed(() => props.historySessions !== null);

const historyItems = computed(() => props.historySessions ?? []);

const historyEmptyText = computed(() => tr(language.value, "noChats"));

const historyPickerRowCount = computed(() =>
  showHistoryPicker.value ? Math.max(historyItems.value.length, 1) : 0,
);

const showModelPicker = computed(() => modelPickerOpen.value);
const showChatModePicker = computed(() => chatModePickerOpen.value);
const showApprovalPicker = computed(() => approvalPickerOpen.value);
const showThinkingTierList = computed(() => thinkingTierPickerOpen.value);

const modelPickerRowCount = computed(() => {
  if (!showModelPicker.value) {
    return 0;
  }
  const models = Math.max(modelPickerModels.value.length, 1);
  const groups = Math.max(groupModelsByProvider(modelPickerModels.value).length, 1);
  // group headers + model rows + refresh
  return groups + models + 1;
});

const chatModePickerRowCount = computed(() =>
  showChatModePicker.value ? chatModePickerOptions.value.length : 0,
);

const approvalPickerRowCount = computed(() =>
  showApprovalPicker.value ? approvalPickerOptions.value.length : 0,
);

const thinkingTierPickerRowCount = computed(() =>
  showThinkingTierList.value ? thinkingTierPickerOptions.value.length : 0,
);

// function historySlug(sessionId: string) {
//   const compact = sessionId.replace(/^session-/, "").slice(-8) || sessionId;
//   return compact.length > 12 ? `${compact.slice(0, 12)}…` : compact;
// }

const showPathPermissionPicker = computed(() => Boolean(props.pathPermission));

const pathPermissionHeader = computed(() => tr(language.value, "permissionRequest"));

const pathPermissionQuestion = computed(() => {
  const operation = props.pathPermission?.operation ?? "write";
  const tool = props.pathPermission?.toolName ?? "tool";
  return tr(language.value, "permissionQuestion", {
    operation: tr(language.value, operation === "write" ? "write" : "read"),
    tool,
  });
});

const pathPermissionOptions = computed(() => [
  { slug: "yes", label: tr(language.value, "allowOnce"), description: tr(language.value, "allowOnceDesc"), decision: "allow_once" as const },
  { slug: "always", label: tr(language.value, "allowAlways"), description: tr(language.value, "allowAlwaysDesc"), decision: "allow_always" as const },
  { slug: "no", label: tr(language.value, "deny"), description: tr(language.value, "denyDesc"), decision: "deny" as const },
]);

const showToolApprovalPicker = computed(() => Boolean(props.toolApproval));

const toolApprovalHeader = computed(() => tr(language.value, "toolApprovalTitle"));

const toolApprovalOptions = computed(() => [
  {
    slug: "once",
    label: tr(language.value, "allowOnce"),
    description: tr(language.value, "allowOnceDesc"),
    decision: "allow_once" as const,
  },
  {
    slug: "session",
    label: tr(language.value, "allowSession"),
    description: tr(language.value, "allowSessionDesc"),
    decision: "allow_session" as const,
  },
  {
    slug: "deny",
    label: tr(language.value, "deny"),
    description: tr(language.value, "denyDesc"),
    decision: "deny" as const,
  },
]);

const showAskUserPicker = computed(
  () =>
    Boolean(
      props.askUser &&
        props.askUser.questions.length > 0 &&
        !askUserFinishing.value,
    ),
);

const askQuestionCount = computed(() => props.askUser?.questions.length ?? 0);

const activeAskQuestion = computed(
  () => props.askUser?.questions[askQuestionIndex.value],
);

function toAskSlug(label: string) {
  return label
    .trim()
    .toLowerCase()
    .replace(/[^\w\u4e00-\u9fff]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 32);
}

const skipAskOption = computed<AskDisplayOption>(() => ({
  label: tr(language.value, "customAnswer"),
  slug: "custom",
  description: tr(language.value, "customAnswerDesc"),
  isSkip: true,
}));

const activeAskOptions = computed<AskDisplayOption[]>(() => {
  const options = (activeAskQuestion.value?.options ?? []).map((option) => ({
    label: option.label,
    slug: toAskSlug(option.label) || "option",
    description: option.description,
  }));
  return [...options, skipAskOption.value];
});

const askConfirmRowIndex = computed(() =>
  activeAskQuestion.value?.multiSelect ? activeAskOptions.value.length : -1,
);

const pathPermissionPickerRowCount = computed(() =>
  showPathPermissionPicker.value ? 3 + pathPermissionOptions.value.length : 0,
);

const toolApprovalPickerRowCount = computed(() =>
  showToolApprovalPicker.value ? 1 + toolApprovalOptions.value.length : 0,
);

const interactivePickerOpen = computed(
  () =>
    showAskUserPicker.value ||
    showPathPermissionPicker.value ||
    showToolApprovalPicker.value ||
    showHistoryPicker.value ||
    showModelPicker.value ||
    showChatModePicker.value ||
    showApprovalPicker.value ||
    showThinkingTierList.value ||
    workspacePickerOpen.value,
);

/** Pickers that must keep the input read-only (model picker allows typing to filter). */
const inputLockedForTyping = computed(
  () =>
    showAskUserPicker.value ||
    showPathPermissionPicker.value ||
    showToolApprovalPicker.value ||
    showHistoryPicker.value ||
    showChatModePicker.value ||
    showApprovalPicker.value ||
    showThinkingTierList.value ||
    workspacePickerOpen.value,
);

const askPickerRowCount = computed(() => {
  if (!showAskUserPicker.value) {
    return 0;
  }
  const optionRows = activeAskOptions.value.length;
  const confirmRow = activeAskQuestion.value?.multiSelect ? 1 : 0;
  return 2 + optionRows + confirmRow;
});

const pastedLineCount = computed(() =>
  pastedText.value ? pastedText.value.split(/\r\n|\r|\n/).length : 0,
);
const hasInlineAttachmentTags = computed(
  () =>
    Boolean(props.selectionLines) ||
    Boolean(pastedText.value) ||
    mentionedFiles.value.length > 0 ||
    attachedFiles.value.length > 0,
);
const hasAttachmentTags = computed(
  () => hasInlineAttachmentTags.value || attachedImages.value.length > 0,
);

const inputPlaceholder = computed(() => {
  // Images sit above the text field — keep the hint when only images are attached.
  if (prefixText.value || hasInlineAttachmentTags.value) {
    return "";
  }
  if (props.sending && !interactivePickerOpen.value) {
    return canSend.value
      ? tr(language.value, "attachInjectHint")
      : tr(language.value, "aiResponding");
  }
  if (showHistoryPicker.value) {
    return tr(language.value, "openChatHint");
  }
  if (showModelPicker.value) {
    return tr(language.value, "selectModelHint");
  }
  if (showChatModePicker.value || showApprovalPicker.value || showThinkingTierList.value) {
    return tr(language.value, "selectOptionHint");
  }
  if (showPathPermissionPicker.value || showToolApprovalPicker.value) {
    return tr(language.value, "permissionHint");
  }
  if (showAskUserPicker.value) {
    return tr(language.value, activeAskQuestion.value?.multiSelect ? "askHint" : "askCustomHint");
  }
  return props.placeholder || tr(language.value, "askAnything");
});

const canSend = computed(() =>
  prefixText.value.trim().length > 0 ||
  message.value.trim().length > 0 ||
  pastedText.value.length > 0 ||
  mentionedFiles.value.length > 0 ||
  attachedFiles.value.length > 0 ||
  attachedImages.value.length > 0,
);

const showPauseIcon = computed(() => props.sending && !canSend.value);

function composeVisibleText() {
  const pre = prefixText.value;
  const post = message.value;
  if (!pre) return post;
  if (!post) return pre;
  if (/\s$/.test(pre) || /^\s/.test(post)) return `${pre}${post}`;
  return `${pre} ${post}`;
}

/** Move typed text before the first attachment tag so chips sit mid-line. */
function lockPrefixFromMessage() {
  if (!message.value || hasAttachmentTags.value || prefixText.value) {
    return;
  }
  prefixText.value = message.value;
  message.value = "";
}

function collapsePrefixIfNeeded() {
  if (hasAttachmentTags.value || !prefixText.value) {
    return;
  }
  message.value = composeVisibleText();
  prefixText.value = "";
}

function removeTrailingAttachment(): boolean {
  if (attachedImages.value.length > 0) {
    attachedImages.value.pop();
    collapsePrefixIfNeeded();
    return true;
  }
  if (attachedFiles.value.length > 0) {
    attachedFiles.value.pop();
    collapsePrefixIfNeeded();
    return true;
  }
  if (mentionedFiles.value.length > 0) {
    mentionedFiles.value.pop();
    collapsePrefixIfNeeded();
    return true;
  }
  if (pastedText.value) {
    pastedText.value = "";
    collapsePrefixIfNeeded();
    return true;
  }
  if (props.selectionLines) {
    emit("removeSelection");
    // selectionLines is a prop; collapse on next tick after parent clears it
    void nextTick(() => collapsePrefixIfNeeded());
    return true;
  }
  return false;
}

let layoutChangeFlushScheduled = false;

function emitLayoutChange() {
  if (layoutChangeFlushScheduled) {
    return;
  }
  layoutChangeFlushScheduled = true;
  void nextTick(() => {
    layoutChangeFlushScheduled = false;
    flushLayoutChange();
  });
}


/** Last measured picker list height — refined after paint so tall/desc rows fit. */
let measuredPickerHeight = 0;
let pickerMeasureScheduled = false;

function estimateActivePickerHeight(pickerRows: number): number {
  if (pickerRows <= 0) {
    return 0;
  }
  // Match component row metrics (padding + row). Prefer overestimate to avoid clipping.
  if (showChatModePicker.value) {
    return 10 + chatModePickerOptions.value.length * 48;
  }
  if (showApprovalPicker.value) {
    return 10 + approvalPickerOptions.value.length * 36;
  }
  if (showThinkingTierList.value) {
    return 10 + thinkingTierPickerOptions.value.length * 36;
  }
  if (showModelPicker.value) {
    const models = Math.max(modelPickerModels.value.length, 1);
    const groups = Math.max(
      groupModelsByProvider(modelPickerModels.value).length,
      1,
    );
    return 6 + groups * 24 + models * 32 + 34;
  }
  if (showHistoryPicker.value) {
    return 10 + Math.max(historyItems.value.length, 1) * 32;
  }
  if (showAskUserPicker.value) {
    const options =
      activeAskOptions.value.length +
      (activeAskQuestion.value?.multiSelect ? 1 : 0);
    return 10 + 26 + 48 + options * 30;
  }
  if (showPathPermissionPicker.value) {
    return 10 + 26 + 48 + 34 + pathPermissionOptions.value.length * 30;
  }
  if (showToolApprovalPicker.value) {
    return 10 + 26 + toolApprovalOptions.value.length * 30;
  }
  if (workspacePickerOpen.value) {
    return 10 + pickerRows * 32;
  }
  if (showSuggestions.value) {
    return 9 + suggestionCount.value * 30;
  }
  return 9 + pickerRows * 32;
}

function activePickerRowCount(): number {
  if (workspacePickerOpen.value) return workspacePickerRowCount.value;
  if (showAskUserPicker.value) return askPickerRowCount.value;
  if (showPathPermissionPicker.value) return pathPermissionPickerRowCount.value;
  if (showToolApprovalPicker.value) return toolApprovalPickerRowCount.value;
  if (showHistoryPicker.value) return historyPickerRowCount.value;
  if (showModelPicker.value) return modelPickerRowCount.value;
  if (showChatModePicker.value) return chatModePickerRowCount.value;
  if (showApprovalPicker.value) return approvalPickerRowCount.value;
  if (showThinkingTierList.value) return thinkingTierPickerRowCount.value;
  if (showSuggestions.value) return suggestionCount.value;
  return 0;
}

function schedulePickerHeightMeasure() {
  if (pickerMeasureScheduled) {
    return;
  }
  pickerMeasureScheduled = true;
  void nextTick(async () => {
    pickerMeasureScheduled = false;
    // Wait two frames so Transition/GSAP has mounted the list.
    await new Promise<void>((resolve) =>
      requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
    );
    if (activePickerRowCount() <= 0) {
      measuredPickerHeight = 0;
      return;
    }
    const list = document.querySelector(
      ".chat-input-shell .command-list",
    ) as HTMLElement | null;
    const height = list?.offsetHeight ?? 0;
    if (height <= 0) {
      return;
    }
    if (Math.abs(height - measuredPickerHeight) < 1) {
      return;
    }
    measuredPickerHeight = height;
    flushLayoutChange();
  });
}

function flushLayoutChange() {
  const pickerRows = activePickerRowCount();
  if (pickerRows <= 0) {
    measuredPickerHeight = 0;
  }

  const pickerHeight =
    pickerRows > 0
      ? Math.max(measuredPickerHeight, estimateActivePickerHeight(pickerRows))
      : 0;

  emit("layoutChange", {
    showSuggestions: showSuggestions.value,
    suggestionCount: suggestionCount.value,
    showModelMenu: false,
    modelMenuHeight: 0,
    askUserRowCount: showAskUserPicker.value ? askPickerRowCount.value : 0,
    pickerRowCount: pickerRows,
    pickerHeight,
    // Grow the overlay for image thumbs and/or file chips above the input.
    hasImages: attachedImages.value.length > 0,
    hasFiles: attachedFiles.value.length > 0,
  });

  if (pickerRows > 0) {
    schedulePickerHeightMeasure();
  }
}

async function syncPopupState(open: boolean) {
  const windowLabel = getCurrentWebviewWindow().label;
  try {
    await setOverlayPopupOpen(windowLabel, open);
  } catch (error) {
    console.error("set_overlay_popup_open failed:", error);
  }
}

function closeChipPickers() {
  if (modelPickerOpen.value) {
    endModelFilterSession();
  }
  modelPickerOpen.value = false;
  approvalPickerOpen.value = false;
  chatModePickerOpen.value = false;
  thinkingTierPickerOpen.value = false;
}

function closeModelPicker() {
  if (!modelPickerOpen.value) {
    return;
  }
  endModelFilterSession();
  modelPickerOpen.value = false;
  if (!approvalPickerOpen.value && !chatModePickerOpen.value && !thinkingTierPickerOpen.value) {
    void syncPopupState(false);
  }
  emitLayoutChange();
}

function closeApprovalPicker() {
  if (!approvalPickerOpen.value) {
    return;
  }
  approvalPickerOpen.value = false;
  if (!modelPickerOpen.value && !chatModePickerOpen.value && !thinkingTierPickerOpen.value) {
    void syncPopupState(false);
  }
  emitLayoutChange();
}

function closeChatModePicker() {
  if (!chatModePickerOpen.value) {
    return;
  }
  chatModePickerOpen.value = false;
  if (!modelPickerOpen.value && !approvalPickerOpen.value && !thinkingTierPickerOpen.value) {
    void syncPopupState(false);
  }
  emitLayoutChange();
}

function closeThinkingTierPicker() {
  if (!thinkingTierPickerOpen.value) {
    return;
  }
  thinkingTierPickerOpen.value = false;
  if (!modelPickerOpen.value && !approvalPickerOpen.value && !chatModePickerOpen.value) {
    void syncPopupState(false);
  }
  emitLayoutChange();
}

/** Compatibility aliases used by shared close sites. */
function closeApprovalMenu(_immediate = false) {
  closeApprovalPicker();
}
function closeChatModeMenu(_immediate = false) {
  closeChatModePicker();
}
function closeThinkingTierMenu(_immediate = false) {
  closeThinkingTierPicker();
}

async function prepareChipPicker() {
  if (showHistoryPicker.value) {
    emit("historyClose");
  }
  workspacePickerOpen.value = false;
  workspaceQuickSelectOnly.value = false;
  closeChipPickers();
}

async function openModelPicker() {
  await prepareChipPicker();
  beginModelFilterSession();
  const currentIdx = modelPickerModels.value.findIndex((model) =>
    isModelEntrySelected(model, chatModel.value, chatModelProvider.value),
  );
  selectedIndex.value = currentIdx >= 0 ? currentIdx : 0;
  modelPickerOpen.value = true;
  await syncPopupState(true);
  emitLayoutChange();
  void focusInput();

  if (chatModelStore.models.length === 0) {
    void chatModelStore.fetch().then(() => {
      if (modelPickerOpen.value) {
        emitLayoutChange();
      }
    });
  } else {
    void chatModelStore.softRefresh().then(() => {
      if (modelPickerOpen.value) {
        emitLayoutChange();
      }
    });
  }
}

async function openApprovalPicker() {
  if (settingStore.chatMode === "ask") {
    return;
  }
  await prepareChipPicker();
  const idx = approvalPickerOptions.value.findIndex(
    (option) => option.id === settingStore.toolApprovalMode,
  );
  selectedIndex.value = idx >= 0 ? idx : 0;
  approvalPickerOpen.value = true;
  await syncPopupState(true);
  emitLayoutChange();
  void focusInput();
}

async function openChatModePicker() {
  await prepareChipPicker();
  const idx = chatModePickerOptions.value.findIndex(
    (option) => option.id === settingStore.chatMode,
  );
  selectedIndex.value = idx >= 0 ? idx : 0;
  chatModePickerOpen.value = true;
  await syncPopupState(true);
  emitLayoutChange();
  void focusInput();
}

async function openThinkingTierPicker() {
  if (!showThinkingTierPicker.value) {
    return;
  }
  await prepareChipPicker();
  const idx = thinkingTierPickerOptions.value.findIndex(
    (option) => option.id === chatModel.value,
  );
  selectedIndex.value = idx >= 0 ? idx : 0;
  thinkingTierPickerOpen.value = true;
  await syncPopupState(true);
  emitLayoutChange();
  void focusInput();
}

function toggleModelMenu() {
  if (modelPickerOpen.value) {
    closeModelPicker();
    return;
  }
  void openModelPicker();
}

function toggleApprovalMenu() {
  if (settingStore.chatMode === "ask") {
    return;
  }
  if (approvalPickerOpen.value) {
    closeApprovalPicker();
    return;
  }
  void openApprovalPicker();
}

function toggleChatModeMenu() {
  if (chatModePickerOpen.value) {
    closeChatModePicker();
    return;
  }
  void openChatModePicker();
}

function toggleThinkingTierMenu() {
  if (!showThinkingTierPicker.value) {
    return;
  }
  if (thinkingTierPickerOpen.value) {
    closeThinkingTierPicker();
    return;
  }
  void openThinkingTierPicker();
}

function flashModelChipConfirm() {
  modelChipConfirm.value = true;
  if (modelChipConfirmTimer) {
    clearTimeout(modelChipConfirmTimer);
  }
  modelChipConfirmTimer = setTimeout(() => {
    modelChipConfirm.value = false;
    modelChipConfirmTimer = null;
  }, 120);
}

function selectModel(entry: ChatModelInfo) {
  closeModelPicker();
  const nextId = entry.id;
  const nextProvider = entry.provider;
  if (nextId === chatModel.value && nextProvider === chatModelProvider.value) {
    return;
  }
  chatModel.value = nextId;
  chatModelProvider.value = nextProvider;
  flashModelChipConfirm();
  void settingStore.update({ chatModel: nextId, chatModelProvider: nextProvider });
  emit("modelChange", nextId);
}

async function refreshModelList() {
  await chatModelStore.reload();
  if (modelPickerOpen.value) {
    emitLayoutChange();
  }
}

function selectApprovalMode(mode: string) {
  closeApprovalPicker();
  const next = mode as ToolApprovalMode;
  if (next === settingStore.toolApprovalMode) {
    return;
  }
  void settingStore.update({ toolApprovalMode: next });
}

function selectChatMode(mode: string) {
  closeChatModePicker();
  const next = mode as ChatMode;
  if (next === settingStore.chatMode) {
    return;
  }
  if (next === "ask") {
    closeApprovalPicker();
  }
  void settingStore.update({ chatMode: next });
}

function selectThinkingTier(variantId: string) {
  closeThinkingTierPicker();
  if (variantId === chatModel.value) {
    return;
  }
  chatModel.value = variantId;
  void settingStore.update({
    chatModel: variantId,
    chatModelProvider: chatModelProvider.value,
  });
  emit("modelChange", variantId);
}

onMounted(async () => {
  console.debug("slash command registration", {
    commands: slashCommands.map((item) => item.command),
    available: props.enableCommands && props.contextReady,
  });
  await chatModelStore.fetch();
  if (
    chatModelStore.models.length === 0 &&
    chatModel.value.trim() === "deepseek-chat"
  ) {
    chatModel.value = "";
    chatModelProvider.value = "";
    if (settingStore.chatModel.trim() === "deepseek-chat") {
      void settingStore.update({ chatModel: "", chatModelProvider: "" });
    }
  } else if (
    chatModelStore.models.length > 0 &&
    (!chatModel.value.trim() ||
      !isKnownModelSelection(
        chatModelStore.models,
        chatModel.value,
        chatModelProvider.value,
      ))
  ) {
    const fallbackId = chatModelStore.models[0].id;
    const fallbackProvider = chatModelStore.models[0].provider;
    chatModel.value = fallbackId;
    chatModelProvider.value = fallbackProvider;
    if (
      fallbackId !== settingStore.chatModel ||
      fallbackProvider !== settingStore.chatModelProvider
    ) {
      void settingStore.update({
        chatModel: fallbackId,
        chatModelProvider: fallbackProvider,
      });
    }
  }
  void refreshContextUsage();
  await loadWorkspaceState();
  unlistenWorkspaces = await listen("workspaces-changed", () => {
    void loadWorkspaceState();
  });
  unlistenChatFinished = await listen("chat-finished", () => {
    void refreshContextUsage();
  });
  unlistenChatStarted = await listen("chat-started", () => {
    void refreshContextUsage();
  });
  try {
    unlistenFocus = await getCurrentWebviewWindow().onFocusChanged(({ payload: focused }) => {
      if (focused) {
        restorePickerFocus();
      }
    });
  } catch (error) {
    console.error("onFocusChanged failed:", error);
  }
});

onUnmounted(() => {
  unlistenWorkspaces?.();
  unlistenChatFinished?.();
  unlistenChatStarted?.();
  unlistenFocus?.();
  void syncPopupState(false);
});

useEventListener(window, "keydown", handleGlobalKeydown);
useEventListener(window, "focus", restorePickerFocus);
useEventListener(document, "visibilitychange", () => {
  if (document.visibilityState === "visible") {
    restorePickerFocus();
  }
});

const isCommandMode = computed(
  () =>
    !interactivePickerOpen.value &&
    props.enableCommands &&
    props.contextReady &&
    message.value.startsWith("/") &&
    !message.value.includes(" "),
);

watch(
  () => [props.enableCommands, props.contextReady] as const,
  ([enabled, ready]) => {
    console.debug("slash command availability", {
      enabled,
      contextReady: ready,
      available: enabled && ready,
    });
  },
  { immediate: true },
);

const filteredCommands = computed(() => {
  if (!isCommandMode.value) {
    return [];
  }

  const query = message.value.toLowerCase();
  return slashCommands
    .filter(
      (item) =>
        item.command.toLowerCase().startsWith(query) &&
        (item.command !== "/work" || props.showWorkspaceButton),
    )
    .map((item) => ({
      ...item,
      description: tr(language.value, item.descriptionKey),
    }));
});

const showCommandSuggestions = computed(
  () => isCommandMode.value && filteredCommands.value.length > 0,
);

const workspaceFiles = ref<string[]>([]);
const workspaceFilesLoading = ref(false);
const workspaceFilesRoot = ref("");
const activeFileMention = computed(() => {
  if (
    !currentWorkspace.value ||
    interactivePickerOpen.value
  ) {
    return null;
  }
  const match = message.value.match(/(?:^|\s)@([^\s]*)$/);
  return match ? { query: match[1], start: match.index! + match[0].indexOf("@") } : null;
});
const fileSuggestions = computed(() => {
  const mention = activeFileMention.value;
  if (!mention) return [];
  const query = mention.query.toLowerCase();
  return workspaceFiles.value
    .filter((path) => path.toLowerCase().includes(query))
    .sort((left, right) => {
      const leftName = left.split("/").pop()?.toLowerCase() ?? left.toLowerCase();
      const rightName = right.split("/").pop()?.toLowerCase() ?? right.toLowerCase();
      const leftRank = leftName.startsWith(query) ? 0 : 1;
      const rightRank = rightName.startsWith(query) ? 0 : 1;
      return leftRank - rightRank || left.length - right.length || left.localeCompare(right);
    })
    .slice(0, 12);
});
const showFileSuggestions = computed(() => activeFileMention.value !== null);
const showSuggestions = computed(
  () => showFileSuggestions.value || showCommandSuggestions.value,
);
const suggestionCount = computed(() =>
  showFileSuggestions.value
    ? Math.max(fileSuggestions.value.length, 1)
    : filteredCommands.value.length,
);

async function ensureWorkspaceFiles() {
  const root = currentWorkspace.value?.root ?? "";
  if (!root || workspaceFilesRoot.value === root) return;
  workspaceFilesLoading.value = true;
  try {
    workspaceFiles.value = await listWorkspaceFiles();
    workspaceFilesRoot.value = root;
  } catch (error) {
    console.error("list_workspace_files failed:", error);
    workspaceFiles.value = [];
    workspaceFilesRoot.value = root;
  } finally {
    workspaceFilesLoading.value = false;
  }
}

function selectWorkspaceFile(path: string) {
  const mention = activeFileMention.value;
  if (!mention) return;
  message.value = message.value.slice(0, mention.start);
  lockPrefixFromMessage();
  if (!mentionedFiles.value.includes(path)) {
    mentionedFiles.value.push(path);
  }
  selectedIndex.value = 0;
  void nextTick(() => focusInput());
}

function fileName(path: string) {
  return path.split("/").pop() || path;
}

async function focusInput() {
  await nextTick();
  inputRef.value?.focus({ preventScroll: true });
}

/** Keyboard navigation for pickers must work even if the input lost focus (e.g. after Alt-Tab). */
function handleGlobalKeydown(event: KeyboardEvent) {
  if (!interactivePickerOpen.value) {
    return;
  }
  if (event.target === inputRef.value) {
    return;
  }
  handleKeydown(event);
}

function restorePickerFocus() {
  if (interactivePickerOpen.value) {
    void focusInput();
  }
}

async function executeCommand(command: string) {
  if (!props.enableCommands || !props.contextReady) {
    console.debug("slash command blocked", { command, contextReady: props.contextReady });
    return;
  }
  message.value = "";
  prefixText.value = "";
  selectedIndex.value = 0;
  emitLayoutChange();
  const action = await executeSlashCommand(command);
  if (action === "openHistory") {
    emit("openHistory");
    return;
  }
  if (action === "openModel") {
    void openModelPicker();
    return;
  }
  if (action === "openWorkspace") {
    await openWorkspaceQuickPicker();
    return;
  }
  if (action === "clearInput") {
    reset();
    return;
  }
  if (action === "showContext") {
    try {
      emit("showContext", await fetchEnvironmentContext());
    } catch (error) {
      console.error("Failed to invoke get_environment_context; using resolved overlay snapshot:", error);
      emit("showContext", props.capturedContext ?? {});
    }
    return;
  }
  if (action === "close") {
    emit("close");
  }
}

// Global workspace selection is available only before a conversation starts.
const workspaces = ref<Workspace[]>([]);
const currentWorkspace = ref<Workspace | null>(null);
const workspacePickerOpen = ref(false);
const workspaceQuickSelectOnly = ref(false);
const workspaceSaving = ref(false);
const workspaceError = ref("");
let unlistenWorkspaces: UnlistenFn | null = null;
let unlistenChatFinished: UnlistenFn | null = null;
let unlistenChatStarted: UnlistenFn | null = null;
let unlistenFocus: UnlistenFn | null = null;

const workspaceTooltip = computed(() =>
  currentWorkspace.value
    ? `${currentWorkspace.value.name}\n${currentWorkspace.value.root}`
    : "Create a workspace before starting a conversation",
);

const workspacePickerRowCount = computed(() => {
  if (!workspacePickerOpen.value) return 0;
  return 2 + Math.max(workspaces.value.length, 1);
});

async function loadWorkspaceState() {
  try {
    const [items, current] = await Promise.all([
      listWorkspaces(),
      getCurrentWorkspace(),
    ]);
    workspaces.value = items;
    if (current?.root !== currentWorkspace.value?.root) {
      workspaceFiles.value = [];
      workspaceFilesRoot.value = "";
    }
    currentWorkspace.value = current;
  } catch (error) {
    workspaceError.value = String(error);
  }
}

async function toggleWorkspacePicker() {
  if (workspacePickerOpen.value) {
    workspacePickerOpen.value = false;
    await syncPopupState(false);
    emitLayoutChange();
    return;
  }
  workspaceError.value = "";
  workspaceQuickSelectOnly.value = false;
  selectedIndex.value = 0;
  closeModelPicker();
  closeApprovalMenu();
  closeChatModeMenu();
  closeThinkingTierMenu();
  await loadWorkspaceState();
  if (workspaces.value.length === 0) {
    await addWorkspaceFromFolder();
    return;
  }
  workspacePickerOpen.value = true;
  await syncPopupState(true);
  emitLayoutChange();
}

async function openWorkspaceQuickPicker() {
  workspaceError.value = "";
  workspaceQuickSelectOnly.value = true;
  selectedIndex.value = 0;
  closeModelPicker();
  closeApprovalMenu();
  closeChatModeMenu();
  closeThinkingTierMenu();
  await loadWorkspaceState();
  workspacePickerOpen.value = true;
  await syncPopupState(true);
  emitLayoutChange();
}

async function addWorkspaceFromFolder() {
  if (workspaceSaving.value) return;
  workspaceSaving.value = true;
  workspaceError.value = "";
  await syncPopupState(true);
  try {
    const root = await selectWorkspaceFolder();
    if (!root) return;
    const workspace = await createWorkspace(root);
    currentWorkspace.value = await switchWorkspace(workspace.id);
    await loadWorkspaceState();
    workspacePickerOpen.value = false;
    await syncPopupState(false);
  } catch (error) {
    workspaceError.value = String(error);
    workspacePickerOpen.value = true;
  } finally {
    workspaceSaving.value = false;
    if (!workspacePickerOpen.value) await syncPopupState(false);
    emitLayoutChange();
  }
}

async function chooseWorkspace(workspace: Workspace) {
  if (workspace.id !== currentWorkspace.value?.id) {
    try {
      currentWorkspace.value = await switchWorkspace(workspace.id);
    } catch (error) {
      workspaceError.value = String(error);
      return;
    }
  }
  workspacePickerOpen.value = false;
  workspaceQuickSelectOnly.value = false;
  await syncPopupState(false);
  emitLayoutChange();
  void focusInput();
}

async function exitCurrentWorkspace() {
  workspaceError.value = "";
  try {
    await clearCurrentWorkspace();
    currentWorkspace.value = null;
    workspacePickerOpen.value = false;
    workspaceQuickSelectOnly.value = false;
    await syncPopupState(false);
    emitLayoutChange();
    void focusInput();
  } catch (error) {
    workspaceError.value = String(error);
  }
}

function selectHistorySession(sessionId: string) {
  emit("historySelect", sessionId);
}

function closeHistoryPicker() {
  emit("historyClose");
}

async function submit() {
  if (workspacePickerOpen.value) {
    return;
  }
  if (props.sending && !canSend.value && !interactivePickerOpen.value) {
    emit("pause");
    return;
  }

  if (showHistoryPicker.value) {
    const item = historyItems.value[selectedIndex.value];
    if (item) {
      selectHistorySession(item.sessionId);
    }
    return;
  }

  if (showModelPicker.value) {
    const models = modelPickerModels.value;
    if (selectedIndex.value < models.length) {
      const model = models[selectedIndex.value];
      if (model) selectModel(model);
    } else {
      void refreshModelList();
    }
    return;
  }

  if (showChatModePicker.value) {
    const option = chatModePickerOptions.value[selectedIndex.value];
    if (option) selectChatMode(option.id);
    return;
  }

  if (showApprovalPicker.value) {
    const option = approvalPickerOptions.value[selectedIndex.value];
    if (option) selectApprovalMode(option.id);
    return;
  }

  if (showThinkingTierList.value) {
    const option = thinkingTierPickerOptions.value[selectedIndex.value];
    if (option) selectThinkingTier(option.id);
    return;
  }

  if (showToolApprovalPicker.value) {
    const option = toolApprovalOptions.value[selectedIndex.value];
    if (option) {
      selectToolApproval(option.decision);
    }
    return;
  }

  if (showPathPermissionPicker.value) {
    const option = pathPermissionOptions.value[selectedIndex.value];
    if (option) {
      selectPathPermission(option.decision);
    }
    return;
  }

  if (showAskUserPicker.value) {
    if (
      activeAskQuestion.value?.multiSelect &&
      selectedIndex.value === askConfirmRowIndex.value
    ) {
      confirmAskSelection();
      return;
    }

    const option = activeAskOptions.value[selectedIndex.value];
    if (option) {
      selectAskOption(option);
    }
    return;
  }

  const text = composeVisibleText().trim();
  if (
    !text &&
    !pastedText.value &&
    mentionedFiles.value.length === 0 &&
    attachedFiles.value.length === 0 &&
    attachedImages.value.length === 0
  ) {
    return;
  }

  if (showFileSuggestions.value) {
    const path = fileSuggestions.value[selectedIndex.value];
    if (path) selectWorkspaceFile(path);
    return;
  }

  if (showCommandSuggestions.value) {
    const command = filteredCommands.value[selectedIndex.value]?.command;
    if (command) {
      await executeCommand(command);
    }
    return;
  }

  if (
    props.enableCommands &&
    props.contextReady &&
    slashCommands.some((item) => item.command === text)
  ) {
    await executeCommand(text);
    return;
  }

  const fileMentions = mentionedFiles.value
    .map((path) => (/\s/.test(path) ? `@"${path}"` : `@${path}`))
    .join(" ");
  const attachedFileBlocks = formatAttachedFilesForMessage(attachedFiles.value);
  const imageTags = attachedImages.value.map(img => `![image](${img})`).join("\n");
  const submittedText = [text, fileMentions, attachedFileBlocks, pastedText.value, imageTags]
    .filter((part) => part.length > 0)
    .join("\n\n");
  emit("submit", submittedText);
  message.value = "";
  prefixText.value = "";
  pastedText.value = "";
  mentionedFiles.value = [];
  attachedFiles.value = [];
  attachedImages.value = [];
  emitLayoutChange();
}

function handlePaste(event: ClipboardEvent) {
  const items = event.clipboardData?.items;
  if (items) {
    const files: File[] = [];
    for (const item of items) {
      if (item.kind !== "file") continue;
      const file = item.getAsFile();
      if (file) files.push(file);
    }
    if (files.length > 0) {
      event.preventDefault();
      void ingestDroppedOrPastedFiles(files);
      return;
    }
  }

  const text = event.clipboardData?.getData("text/plain") ?? "";
  if (!/[\r\n]/.test(text)) return;

  event.preventDefault();
  const normalized = text.replace(/\r\n|\r/g, "\n").trim();
  if (!normalized) return;
  lockPrefixFromMessage();
  pastedText.value = pastedText.value
    ? `${pastedText.value}\n${normalized}`
    : normalized;
}

function selectPathPermission(decision: PathPermissionDecision) {
  emit("pathPermissionComplete", decision);
}

function selectToolApproval(decision: ToolApprovalDecision) {
  emit("toolApprovalComplete", decision);
}

function handleKeydown(event: KeyboardEvent) {
  if ((event.key === "Backspace" || event.key === "Delete") && !showModelPicker.value) {
    const input = inputRef.value;
    const caretAtStart =
      Boolean(input) &&
      input!.selectionStart === 0 &&
      input!.selectionEnd === 0;
    const empty = message.value.length === 0;
    // Backspace at start (or empty) / Delete when empty removes file / text / selection tags.
    const shouldRemoveTag =
      (event.key === "Backspace" && caretAtStart) ||
      (event.key === "Delete" && empty);
    if (shouldRemoveTag && removeTrailingAttachment()) {
      event.preventDefault();
      return;
    }
  }

  if (props.sending && !interactivePickerOpen.value && event.key === "Enter") {
    if (canSend.value) {
      event.preventDefault();
      void submit();
      return;
    }
    // 回复中且无可发送内容：回车不暂停（点暂停按钮）
    event.preventDefault();
    return;
  }

  if (workspacePickerOpen.value) {
    if (event.key === "Escape") {
      event.preventDefault();
      workspacePickerOpen.value = false;
      workspaceQuickSelectOnly.value = false;
      void syncPopupState(false);
      emitLayoutChange();
      return;
    }
    const rows = workspaces.value.length + (workspaceQuickSelectOnly.value ? 0 : 1);
    if (rows === 0) return;
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      const delta = event.key === "ArrowDown" ? 1 : -1;
      selectedIndex.value = (selectedIndex.value + delta + rows) % rows;
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      if (!workspaceQuickSelectOnly.value && selectedIndex.value === 0) {
        void addWorkspaceFromFolder();
      } else {
        const workspaceIndex = workspaceQuickSelectOnly.value
          ? selectedIndex.value
          : selectedIndex.value - 1;
        const workspace = workspaces.value[workspaceIndex];
        if (workspace) void chooseWorkspace(workspace);
      }
      return;
    }
  }

  if (showHistoryPicker.value) {
    if (historyItems.value.length === 0) {
      if (event.key === "Escape") {
        event.preventDefault();
        closeHistoryPicker();
      }
      return;
    }
    const totalRows = historyItems.value.length;
    if (event.key === "ArrowDown") {
      event.preventDefault();
      selectedIndex.value = (selectedIndex.value + 1) % Math.max(totalRows, 1);
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      selectedIndex.value =
        (selectedIndex.value - 1 + Math.max(totalRows, 1)) % Math.max(totalRows, 1);
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      const item = historyItems.value[selectedIndex.value];
      if (item) {
        selectHistorySession(item.sessionId);
      }
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      closeHistoryPicker();
      return;
    }
  }

  if (showModelPicker.value) {
    const modelCount = modelPickerModels.value.length;
    const totalRows = modelCount + 1; // models + refresh
    if (event.key === "ArrowDown") {
      event.preventDefault();
      selectedIndex.value = (selectedIndex.value + 1) % Math.max(totalRows, 1);
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      selectedIndex.value =
        (selectedIndex.value - 1 + Math.max(totalRows, 1)) % Math.max(totalRows, 1);
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      if (selectedIndex.value < modelCount) {
        const model = modelPickerModels.value[selectedIndex.value];
        if (model) selectModel(model);
      } else {
        void refreshModelList();
      }
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      closeModelPicker();
      return;
    }
  }

  if (showToolApprovalPicker.value) {
    const totalRows = toolApprovalOptions.value.length;
    if (event.key === "ArrowDown") {
      event.preventDefault();
      selectedIndex.value = (selectedIndex.value + 1) % Math.max(totalRows, 1);
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      selectedIndex.value =
        (selectedIndex.value - 1 + Math.max(totalRows, 1)) % Math.max(totalRows, 1);
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      const option = toolApprovalOptions.value[selectedIndex.value];
      if (option) {
        selectToolApproval(option.decision);
      }
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      return;
    }
  }

  if (showPathPermissionPicker.value) {
    const totalRows = pathPermissionOptions.value.length;
    if (event.key === "ArrowDown") {
      event.preventDefault();
      selectedIndex.value = (selectedIndex.value + 1) % Math.max(totalRows, 1);
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      selectedIndex.value =
        (selectedIndex.value - 1 + Math.max(totalRows, 1)) % Math.max(totalRows, 1);
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      const option = pathPermissionOptions.value[selectedIndex.value];
      if (option) {
        selectPathPermission(option.decision);
      }
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      return;
    }
  }

  if (showAskUserPicker.value) {
    const totalRows =
      activeAskOptions.value.length +
      (activeAskQuestion.value?.multiSelect ? 1 : 0);

    if (event.key === "ArrowDown") {
      event.preventDefault();
      selectedIndex.value = (selectedIndex.value + 1) % Math.max(totalRows, 1);
      return;
    }

    if (event.key === "ArrowUp") {
      event.preventDefault();
      selectedIndex.value =
        (selectedIndex.value - 1 + Math.max(totalRows, 1)) % Math.max(totalRows, 1);
      return;
    }

    if (event.key === "Enter") {
      event.preventDefault();
      if (
        activeAskQuestion.value?.multiSelect &&
        selectedIndex.value === askConfirmRowIndex.value
      ) {
        confirmAskSelection();
        return;
      }
      const option = activeAskOptions.value[selectedIndex.value];
      if (option) {
        selectAskOption(option);
      }
      return;
    }

    if (event.key === "Escape") {
      event.preventDefault();
      return;
    }
  }

  if (showChatModePicker.value) {
    const totalRows = chatModePickerOptions.value.length;
    if (event.key === "ArrowDown") {
      event.preventDefault();
      selectedIndex.value = (selectedIndex.value + 1) % Math.max(totalRows, 1);
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      selectedIndex.value =
        (selectedIndex.value - 1 + Math.max(totalRows, 1)) % Math.max(totalRows, 1);
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      const option = chatModePickerOptions.value[selectedIndex.value];
      if (option) selectChatMode(option.id);
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      closeChatModePicker();
      return;
    }
  }

  if (showApprovalPicker.value) {
    const totalRows = approvalPickerOptions.value.length;
    if (event.key === "ArrowDown") {
      event.preventDefault();
      selectedIndex.value = (selectedIndex.value + 1) % Math.max(totalRows, 1);
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      selectedIndex.value =
        (selectedIndex.value - 1 + Math.max(totalRows, 1)) % Math.max(totalRows, 1);
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      const option = approvalPickerOptions.value[selectedIndex.value];
      if (option) selectApprovalMode(option.id);
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      closeApprovalPicker();
      return;
    }
  }

  if (showThinkingTierList.value) {
    const totalRows = thinkingTierPickerOptions.value.length;
    if (event.key === "ArrowDown") {
      event.preventDefault();
      selectedIndex.value = (selectedIndex.value + 1) % Math.max(totalRows, 1);
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      selectedIndex.value =
        (selectedIndex.value - 1 + Math.max(totalRows, 1)) % Math.max(totalRows, 1);
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      const option = thinkingTierPickerOptions.value[selectedIndex.value];
      if (option) selectThinkingTier(option.id);
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      closeThinkingTierPicker();
      return;
    }
  }

  if (showFileSuggestions.value) {
    const count = fileSuggestions.value.length;
    if (event.key === "ArrowDown" && count > 0) {
      event.preventDefault();
      selectedIndex.value = (selectedIndex.value + 1) % count;
      return;
    }
    if (event.key === "ArrowUp" && count > 0) {
      event.preventDefault();
      selectedIndex.value = (selectedIndex.value - 1 + count) % count;
      return;
    }
    if (event.key === "Tab" || event.key === "Enter") {
      event.preventDefault();
      const path = fileSuggestions.value[selectedIndex.value];
      if (path) selectWorkspaceFile(path);
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      const mention = activeFileMention.value;
      if (mention) message.value = message.value.slice(0, mention.start);
      selectedIndex.value = 0;
      return;
    }
  }

  if (showCommandSuggestions.value) {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      selectedIndex.value = (selectedIndex.value + 1) % filteredCommands.value.length;
      return;
    }

    if (event.key === "ArrowUp") {
      event.preventDefault();
      selectedIndex.value =
        (selectedIndex.value - 1 + filteredCommands.value.length) %
        filteredCommands.value.length;
      return;
    }

    if (event.key === "Tab" || event.key === "Enter") {
      event.preventDefault();
      void submit();
      return;
    }

    if (event.key === "Escape") {
      event.preventDefault();
      message.value = "";
      selectedIndex.value = 0;
      emitLayoutChange();
      return;
    }
  }

  if (event.key === "Enter") {
    event.preventDefault();
    void submit();
    return;
  }

  if (event.key === "Escape" && props.closeOnEscape) {
    event.preventDefault();
    emit("close");
  }
}

function reset() {
  message.value = "";
  prefixText.value = "";
  pastedText.value = "";
  mentionedFiles.value = [];
  attachedFiles.value = [];
  attachedImages.value = [];
  selectedIndex.value = 0;
  closeModelPicker();
  closeApprovalMenu();
  closeChatModeMenu();
  closeThinkingTierMenu();
  workspacePickerOpen.value = false;
  workspaceQuickSelectOnly.value = false;
  emitLayoutChange();
}

function setMessage(text: string) {
  prefixText.value = "";
  pastedText.value = "";
  mentionedFiles.value = [];
  attachedFiles.value = [];
  attachedImages.value = [];
  message.value = text;
  emitLayoutChange();
  void focusInput();
}

function isAskOptionSelected(label: string) {
  if (label === tr(language.value, "customAnswer")) {
    return false;
  }
  const current = askAnswers.value[askQuestionIndex.value] ?? [];
  return current.includes(label);
}

function selectAskOption(option: AskDisplayOption) {
  if (option.isSkip) {
    completeAskUserWithSkip();
    return;
  }

  const question = activeAskQuestion.value;
  if (!question) {
    return;
  }

  if (question.multiSelect) {
    const current = askAnswers.value[askQuestionIndex.value] ?? [];
    askAnswers.value[askQuestionIndex.value] = current.includes(option.label)
      ? current.filter((item) => item !== option.label)
      : [...current, option.label];
    return;
  }

  askAnswers.value[askQuestionIndex.value] = [option.label];
  advanceAskQuestion();
}

function confirmAskSelection() {
  const current = askAnswers.value[askQuestionIndex.value] ?? [];
  if (current.length === 0) {
    return;
  }
  advanceAskQuestion();
}

function completeAskUserWithSkip() {
  if (!props.askUser) {
    return;
  }

  const answers = { ...askAnswers.value };
  for (let index = askQuestionIndex.value; index < props.askUser.questions.length; index += 1) {
    answers[index] = [ASK_SKIP_MARKER];
  }
  finishAskUser(answers, true);
}

function finishAskUser(
  answers: Record<number, string[]>,
  skipped = false,
) {
  if (!props.askUser || askUserFinishing.value) {
    return;
  }

  askUserFinishing.value = true;

  const payload = {
    skipped,
    answers: props.askUser.questions.map((question, index) => {
      const selected = answers[index] ?? [];
      const userSupplement = selected.includes(ASK_SKIP_MARKER);
      return {
        header: question.header,
        question: question.question,
        selected: userSupplement
          ? []
          : selected.filter((item) => item !== ASK_SKIP_MARKER),
        userSupplement,
      };
    }),
  };

  emit("askUserComplete", JSON.stringify(payload));
  emitLayoutChange();
}

function advanceAskQuestion() {
  if (!props.askUser) {
    return;
  }

  if (askQuestionIndex.value < props.askUser.questions.length - 1) {
    askQuestionIndex.value += 1;
    selectedIndex.value = 0;
    emitLayoutChange();
    return;
  }

  finishAskUser(askAnswers.value);
}

watch(
  () => props.askUser?.requestId,
  (requestId) => {
    askUserFinishing.value = false;
    askQuestionIndex.value = 0;
    askAnswers.value = {};
    selectedIndex.value = 0;
    void syncPopupState(Boolean(requestId));
    emitLayoutChange();
  },
);

watch(
  () => props.selectionLines,
  (lines, previous) => {
    if (lines && !previous) {
      lockPrefixFromMessage();
    }
    if (!lines) {
      collapsePrefixIfNeeded();
    }
  },
);

watch(filteredCommands, () => {
  selectedIndex.value = 0;
});

watch(
  () => activeFileMention.value?.query,
  (query) => {
    selectedIndex.value = 0;
    if (query !== undefined) void ensureWorkspaceFiles();
  },
);

watch(fileSuggestions, () => {
  if (selectedIndex.value >= fileSuggestions.value.length) {
    selectedIndex.value = 0;
  }
});

watch(
  [() => chatModelStore.loading, () => chatModelStore.error, modelPickerModels],
  () => {
    if (!modelPickerOpen.value) {
      return;
    }
    const maxIndex = modelPickerModels.value.length; // refresh row
    if (selectedIndex.value > maxIndex) {
      selectedIndex.value = 0;
    }
    emitLayoutChange();
  },
);

watch(
  () => message.value,
  () => {
    if (!modelPickerOpen.value) {
      return;
    }
    const models = modelPickerModels.value;
    const currentIdx = models.findIndex((model) =>
      isModelEntrySelected(model, chatModel.value, chatModelProvider.value),
    );
    selectedIndex.value = currentIdx >= 0 ? currentIdx : 0;
    emitLayoutChange();
  },
);

watch(showSuggestions, () => {
  if (showSuggestions.value && modelPickerOpen.value) {
    closeModelPicker();
  }
  if (showSuggestions.value && approvalPickerOpen.value) {
    closeApprovalMenu();
  }
  if (showSuggestions.value && chatModePickerOpen.value) {
    closeChatModeMenu();
  }
  if (showSuggestions.value && thinkingTierPickerOpen.value) {
    closeThinkingTierMenu();
  }
  emitLayoutChange();
}, { immediate: true });

watch(showAskUserPicker, () => {
  if (showAskUserPicker.value && modelPickerOpen.value) {
    closeModelPicker();
  }
  if (showAskUserPicker.value && approvalPickerOpen.value) {
    closeApprovalMenu();
  }
  if (showAskUserPicker.value && chatModePickerOpen.value) {
    closeChatModeMenu();
  }
  if (showAskUserPicker.value && thinkingTierPickerOpen.value) {
    closeThinkingTierMenu();
  }
  emitLayoutChange();
});

watch(
  () => props.pathPermission,
  async (session) => {
    if (session) {
      selectedIndex.value = 0;
      if (modelPickerOpen.value) {
        closeModelPicker();
      }
      if (approvalPickerOpen.value) {
        closeApprovalMenu();
      }
      if (chatModePickerOpen.value) {
        closeChatModeMenu();
      }
      if (thinkingTierPickerOpen.value) {
        closeThinkingTierMenu();
      }
      await syncPopupState(true);
      void focusInput();
    }
    emitLayoutChange();
  },
);

watch(
  () => props.toolApproval,
  async (session) => {
    if (session) {
      selectedIndex.value = 0;
      if (modelPickerOpen.value) {
        closeModelPicker();
      }
      if (approvalPickerOpen.value) {
        closeApprovalMenu();
      }
      if (chatModePickerOpen.value) {
        closeChatModeMenu();
      }
      if (thinkingTierPickerOpen.value) {
        closeThinkingTierMenu();
      }
      await syncPopupState(true);
      void focusInput();
    }
    emitLayoutChange();
  },
);

watch(
  () => props.historySessions,
  async (sessions) => {
    if (sessions !== null) {
      selectedIndex.value = 0;
      if (modelPickerOpen.value) {
        closeModelPicker();
      }
      if (approvalPickerOpen.value) {
        closeApprovalMenu();
      }
      if (chatModePickerOpen.value) {
        closeChatModeMenu();
      }
      if (thinkingTierPickerOpen.value) {
        closeThinkingTierMenu();
      }
      await syncPopupState(true);
      void focusInput();
    }
    emitLayoutChange();
  },
);
defineExpose({ focusInput, reset, setMessage });
</script>

<style scoped>
.chat-input-shell {
  display: flex;
  flex-direction: column;
}

.input-bar {
  box-sizing: border-box;
  flex: none;
  min-height: 82px;
  padding: 10px 10px 8px 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.input-content,
.input-footer,
.input-footer-primary,
.input-footer-actions {
  display: flex;
  align-items: center;
}

.input-content {
  width: 100%;
  min-height: 28px;
  flex-wrap: wrap;
  gap: 4px;
  row-gap: 3px;
}

.input-prefix {
  flex: none;
  max-width: 55%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--peek-text);
  font-family: var(--peek-font-sans);
  font-size: 14px;
  line-height: 20px;
  cursor: text;
}

.input-footer {
  width: 100%;
  min-height: 28px;
  justify-content: space-between;
  gap: 8px;
  padding-top: 1px;
}

.input-footer-primary {
  min-width: 0;
  gap: 4px;
  flex-wrap: nowrap;
}

.input-footer-actions {
  min-width: 0;
  gap: 6px;
  flex-wrap: nowrap;
  align-items: center;
}

.chat-input {
  flex: 1;
  min-width: 48px;
  margin: 0;
  padding: 0;
  border: 0;
  outline: none;
  background: transparent;
  color: var(--peek-text);
  font-family: var(--peek-font-sans);
  font-size: 14px;
  line-height: 20px;
  caret-color: var(--peek-accent);
}

.chat-input::placeholder {
  color: var(--peek-placeholder);
  transition: color 160ms ease, opacity 160ms ease;
}

.model-picker {
  flex: none;
  min-width: 0;
}

.thinking-tier-slot {
  flex: none;
  min-width: 0;
}

.thinking-tier-slot.dormant {
  display: none;
}

.approval-slot {
  flex: none;
  min-width: 0;
}

.approval-slot.dormant {
  display: none;
}

.footer-chip-icon-slot {
  flex: none;
  width: 13px;
  height: 13px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}

/* Shared ghost-chip language for footer controls */
.footer-chip {
  height: 26px;
  border-radius: 7px;
  border: 1px solid transparent;
  background: transparent;
  color: var(--peek-muted);
  font-family: var(--peek-font-sans);
  font-size: 11px;
  font-weight: 500;
  letter-spacing: 0.01em;
  line-height: 16px;
  transition:
    border-color 160ms ease,
    color 160ms ease,
    background 160ms ease,
    box-shadow 160ms ease;
}

.footer-chip-icon {
  flex: none;
  opacity: 0.78;
  transition: opacity 140ms ease, color 140ms ease;
}

.footer-chip:hover .footer-chip-icon,
.footer-chip.open .footer-chip-icon,
.footer-chip.active .footer-chip-icon {
  opacity: 1;
}

.workspace-control {
  position: relative;
  flex: none;
  height: 26px;
  max-width: 108px;
  min-width: 0;
  display: inline-flex;
  align-items: center;
  border-radius: 6px;
  background: transparent;
  color: var(--peek-muted);
  transition: background 120ms ease, color 120ms ease;
}

.file-mention-tags {
  display: flex;
  flex: none;
  gap: 3px;
  max-width: 42%;
  overflow-x: auto;
  overflow-y: hidden;
}

.file-mention-tag {
  gap: 3px;
  max-width: 120px;
}

.file-mention-name {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.input-bar.drag-over {
  outline: 1px dashed color-mix(in srgb, var(--peek-accent) 55%, transparent);
  outline-offset: -2px;
}

.input-files {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  width: 100%;
  max-height: 64px;
  overflow-x: hidden;
  overflow-y: auto;
  padding: 0;
}

.file-chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  max-width: min(220px, 100%);
  height: 26px;
  padding: 0 4px 0 8px;
  border: 1px solid var(--peek-border);
  border-radius: 7px;
  background: color-mix(in srgb, var(--peek-input-bg) 72%, var(--peek-surface));
  color: var(--peek-text);
  font-size: 12px;
  line-height: 1;
  transition:
    border-color 120ms ease,
    background 120ms ease,
    opacity 120ms ease;
}

.file-chip:hover {
  border-color: color-mix(in srgb, var(--peek-accent) 28%, var(--peek-border));
  background: color-mix(in srgb, var(--peek-accent) 8%, var(--peek-input-bg));
}

.file-chip.skipped {
  opacity: 0.55;
}

.file-chip-icon {
  flex: none;
  color: var(--peek-muted);
}

.file-chip-name {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: 500;
  letter-spacing: 0.01em;
}

.file-chip-remove {
  display: inline-flex;
  flex: none;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  margin: 0;
  padding: 0;
  border: 0;
  border-radius: 5px;
  background: transparent;
  color: var(--peek-muted);
  cursor: pointer;
  opacity: 0.55;
  transition: opacity 120ms ease, background 120ms ease, color 120ms ease;
}

.file-chip:hover .file-chip-remove {
  opacity: 0.9;
}

.file-chip-remove:hover {
  opacity: 1;
  color: var(--peek-text);
  background: color-mix(in srgb, var(--peek-muted) 16%, transparent);
}

.selection-tag {
  display: inline-flex;
  flex: none;
  align-items: center;
  height: 20px;
  margin: 0;
  padding: 0 5px;
  border: 1px solid color-mix(in srgb, var(--peek-accent) 34%, var(--peek-border));
  border-radius: 4px;
  background: color-mix(in srgb, var(--peek-accent) 10%, transparent);
  color: var(--peek-accent);
  font-size: 10px;
  font-weight: 600;
  line-height: 1;
  white-space: nowrap;
}

.workspace-control:hover {
  background: color-mix(in srgb, var(--peek-accent) 12%, transparent);
  color: var(--peek-text);
}

.workspace-control.active {
  background: color-mix(in srgb, var(--peek-accent) 18%, transparent);
  color: var(--peek-accent);
}

.workspace-btn {
  min-width: 0;
  max-width: 100%;
  height: 26px;
  border: 0;
  border-radius: inherit;
  background: transparent;
  color: inherit;
  padding: 0 6px;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: 4px;
  overflow: hidden;
}

.workspace-name {
  min-width: 0;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 11px;
  font-weight: 500;
  line-height: 16px;
}

.workspace-exit-btn {
  position: absolute;
  top: -6px;
  right: -6px;
  z-index: 2;
  width: 17px;
  height: 17px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  border: 1px solid var(--peek-border);
  border-radius: 50%;
  background: var(--peek-surface);
  color: var(--peek-muted);
  cursor: pointer;
  opacity: 0;
  pointer-events: none;
  transform: scale(0.72);
  transition:
    background 120ms ease,
    border-color 120ms ease,
    color 120ms ease,
    opacity 120ms ease,
    transform 120ms ease;
}

.workspace-control:hover .workspace-exit-btn,
.workspace-control:focus-within .workspace-exit-btn {
  opacity: 1;
  pointer-events: auto;
  transform: scale(1);
}

.workspace-exit-btn:hover {
  border-color: var(--destructive);
  background: var(--destructive);
  color: white;
}

.model-badge {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  max-width: 148px;
  min-width: 0;
  margin: 0;
  padding: 0 7px;
  user-select: none;
  cursor: pointer;
  appearance: none;
}

.model-badge > svg,
.model-badge .footer-chip-icon {
  flex: none;
}

.model-name {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  line-height: 16px;
  animation: model-name-fade 160ms ease;
}

@keyframes model-name-fade {
  from {
    opacity: 0.45;
  }
  to {
    opacity: 1;
  }
}

.model-badge.confirm {
  border-color: color-mix(in srgb, var(--peek-accent) 45%, transparent);
  box-shadow: 0 0 0 1px color-mix(in srgb, var(--peek-accent) 22%, transparent);
  color: var(--peek-text);
}

.model-chevron {
  flex: none;
  opacity: 0.45;
  transition: transform 160ms ease, opacity 140ms ease;
}

.model-badge:hover .model-chevron,
.model-badge.open .model-chevron {
  opacity: 0.8;
}

.model-badge.open .model-chevron {
  transform: rotate(180deg);
}

.model-badge:hover,
.model-badge.open {
  border-color: color-mix(in srgb, var(--peek-border) 80%, transparent);
  color: var(--peek-text);
  background: color-mix(in srgb, var(--peek-text) 5%, transparent);
}

.model-badge.open {
  border-color: color-mix(in srgb, var(--peek-accent) 28%, transparent);
  background: color-mix(in srgb, var(--peek-accent) 10%, transparent);
  box-shadow: inset 0 0 0 0.5px color-mix(in srgb, var(--peek-accent) 12%, transparent);
}

.context-label {
  flex: none;
  height: 26px;
  display: inline-flex;
  align-items: center;
  padding: 0 7px;
  border-radius: 7px;
  background: color-mix(in srgb, var(--peek-text) 4%, transparent);
  color: var(--peek-muted);
  font-family: var(--peek-font-sans);
  font-size: 10px;
  font-weight: 500;
  letter-spacing: 0.02em;
  line-height: 16px;
  user-select: none;
}

.send-btn {
  flex: none;
  width: 26px;
  height: 26px;
  border: 0;
  border-radius: 50%;
  background: var(--peek-send-bg);
  color: var(--peek-send-fg);
  padding: 0;
  cursor: default;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  transform: translateZ(0);
  transition:
    background 120ms ease,
    color 120ms ease,
    transform 140ms cubic-bezier(0.22, 1, 0.36, 1);
}

.send-btn svg {
  width: 18px;
  height: 18px;
}

.send-btn.active {
  background: var(--peek-send-active-bg);
  color: var(--peek-send-active-fg);
  cursor: pointer;
}

.send-btn.pause {
  background: color-mix(in srgb, var(--destructive) 18%, var(--peek-send-bg));
  color: color-mix(in srgb, var(--destructive) 85%, var(--peek-send-fg));
  cursor: pointer;
}

.send-btn.active:hover:not(:disabled),
.send-btn.pause:hover:not(:disabled) {
  transform: scale(1.03);
}

.send-btn.active:active:not(:disabled),
.send-btn.pause:active:not(:disabled) {
  transform: scale(0.97);
}

@media (prefers-reduced-motion: reduce) {
  .send-btn,
  .send-btn.active:hover:not(:disabled),
  .send-btn.pause:hover:not(:disabled),
  .send-btn.active:active:not(:disabled),
  .send-btn.pause:active:not(:disabled) {
    transition: none;
    transform: none;
  }
}
</style>

<style>
/* Image thumbnail area in input bar */
.input-images {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin: 0;
  width: 100%;
  padding: 1px 0 2px;
  max-height: 84px;
  overflow-y: auto;
}

.image-thumb-container {
  position: relative;
  flex: none;
  width: 52px;
  height: 52px;
  border-radius: 10px;
  overflow: hidden;
  border: 1px solid var(--peek-border);
  background: color-mix(in srgb, var(--peek-surface) 70%, transparent);
  /* WebView2: force content to clip to radius */
  transform: translateZ(0);
  transition: border-color 140ms ease;
}

.image-thumb-container:hover {
  border-color: color-mix(in srgb, var(--peek-accent) 55%, var(--peek-border));
}

.image-thumb {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: cover;
  border-radius: inherit;
  cursor: zoom-in;
}

.image-remove-btn {
  position: absolute;
  top: 2px;
  right: 2px;
  width: 15px;
  height: 15px;
  border-radius: 50%;
  background: rgba(0, 0, 0, 0.55);
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  border: none;
  padding: 0;
  opacity: 0.8;
  transition: opacity 120ms ease, background 120ms ease;
}

.image-thumb-container:hover .image-remove-btn {
  opacity: 1;
  background: rgba(0, 0, 0, 0.75);
}

.image-remove-btn:hover {
  background: rgba(239, 68, 68, 0.9) !important; /* soft red on hover */
}
</style>

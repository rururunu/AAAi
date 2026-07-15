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
        :header="historyHeader"
        :empty-text="historyEmptyText"
        :ariaLabel="tr(language, 'chatHistory')"
        :format-time="formatTime"
        @hover="selectedIndex = $event"
        @select="selectHistorySession"
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

    <ModelMenu
      ref="modelMenuComponentRef"
      :open="modelMenuOpen"
      :style="modelMenuStyle"
      :ariaLabel="tr(language, 'chooseModel')"
      :loading="chatModelStore.loading"
      :error="chatModelStore.error"
      :models="availableModels"
      :selected-model-id="chatModel"
      :loading-text="modelStatusText.loading"
      :empty-text="modelStatusText.empty"
      @select="selectModel"
    />

    <ApprovalModeMenu
      ref="approvalMenuComponentRef"
      :open="approvalMenuOpen"
      :style="approvalMenuStyle"
      :ariaLabel="tr(language, 'toolApprovalMode')"
      :options="approvalModeOptions"
      :selected-value="settingStore.toolApprovalMode"
      @select="selectApprovalMode"
    />

    <div class="input-bar">
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
        :readonly="interactivePickerOpen"
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

      <div ref="modelTriggerRef" class="model-picker">
        <button
          type="button"
          class="model-badge"
          data-tauri-drag-region="false"
          :class="{ open: modelMenuOpen }"
          :title="modelBadgeTitle"
          :aria-label="modelBadgeTitle"
          aria-haspopup="listbox"
          :aria-expanded="modelMenuOpen"
          @mousedown.stop
          @click.stop="toggleModelMenu"
        >
          <span class="model-name">{{ chatModel }}</span>
          <ChevronDown :size="12" class="model-chevron" />
        </button>
      </div>

      <div ref="approvalTriggerRef" class="model-picker">
        <button
          type="button"
          class="model-badge"
          data-tauri-drag-region="false"
          :class="{ open: approvalMenuOpen }"
          :title="approvalBadgeTitle"
          :aria-label="approvalBadgeTitle"
          aria-haspopup="listbox"
          :aria-expanded="approvalMenuOpen"
          @mousedown.stop
          @click.stop="toggleApprovalMenu"
        >
          <span class="model-name">{{ approvalModeLabel }}</span>
          <ChevronDown :size="12" class="model-chevron" />
        </button>
      </div>
        </div>

        <div class="input-footer-actions">
          <slot name="actions" />

      <motion.button
        type="button"
        class="send-btn"
        data-tauri-drag-region="false"
        :class="sending ? 'pause' : canSend ? 'active' : ''"
        :aria-label="tr(language, sending ? 'pause' : 'send')"
        :disabled="interactivePickerOpen"
        :while-hover="!interactivePickerOpen && (sending || canSend) ? { scale: 1.08 } : undefined"
        :while-press="!interactivePickerOpen && (sending || canSend) ? { scale: 0.92 } : undefined"
        @click="submit"
      >
        <svg v-if="!sending" viewBox="0 0 16 16" fill="none" aria-hidden="true">
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
      </motion.button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { onClickOutside, useEventListener } from "@vueuse/core";
import { storeToRefs } from "pinia";
import { motion } from "motion-v";
import { gsapMenuEnter, gsapMenuLeave, gsapMenuPrepare, gsapPickerEnter, gsapPickerLeave } from "@/services/motion/gsapPresets";
import { ChevronDown, File, Folder, X } from "@lucide/vue";
import HistoryPicker from "./input/HistoryPicker.vue";
import AskUserPicker from "./input/AskUserPicker.vue";
import PathPermissionPicker from "./input/PathPermissionPicker.vue";
import ToolApprovalPicker from "./input/ToolApprovalPicker.vue";
import FileMentionPicker from "./input/FileMentionPicker.vue";
import CommandSuggestions from "./input/CommandSuggestions.vue";
import WorkspacePickerPanel from "./input/WorkspacePickerPanel.vue";
import ModelMenu from "./input/ModelMenu.vue";
import ApprovalModeMenu from "./input/ApprovalModeMenu.vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { executeSlashCommand, slashCommands } from "@/commands/slash";
import { setOverlayPopupOpen } from "@/services/ipc";
import { tr } from "@/services/i18n";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useChatModelStore } from "@/stores/chatModel";
import { useSettingStore } from "@/stores/setting";
import {
  localizedOptionLabel,
  toolApprovalModeOptions,
  type ToolApprovalMode,
} from "@/types/setting";
import type {
  AskDisplayOption,
  AskUserQuestion,
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
  }>(),
  {
    sending: false,
    placeholder: "",
    enableCommands: true,
    closeOnEscape: true,
    showWorkspaceButton: false,
    selectionLines: 0,
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
  enterPlan: [];
  layoutChange: [
    payload: {
      showSuggestions: boolean;
      suggestionCount: number;
      showModelMenu: boolean;
      modelMenuHeight: number;
      askUserRowCount: number;
      pickerRowCount: number;
    },
  ];
  modelChange: [modelId: string];
}>();

const MODEL_MENU_GAP = 6;
const MODEL_MENU_MIN_WIDTH = 220;

const message = ref("");
const prefixText = ref("");
const pastedText = ref("");
const mentionedFiles = ref<string[]>([]);
const inputRef = ref<HTMLInputElement | null>(null);
const modelTriggerRef = ref<HTMLElement | null>(null);
const modelMenuComponentRef = ref<InstanceType<typeof ModelMenu> | null>(null);
const modelMenuStyle = ref<Record<string, string>>({
  visibility: "hidden",
  pointerEvents: "none",
});
const modelMenuHeight = ref(0);
const approvalTriggerRef = ref<HTMLElement | null>(null);
const approvalMenuComponentRef = ref<InstanceType<typeof ApprovalModeMenu> | null>(null);
const approvalMenuStyle = ref<Record<string, string>>({
  visibility: "hidden",
  pointerEvents: "none",
});
const approvalMenuHeight = ref(0);

function getModelMenuEl(): HTMLElement | null {
  return modelMenuComponentRef.value?.menuEl ?? null;
}

function getApprovalMenuEl(): HTMLElement | null {
  return approvalMenuComponentRef.value?.menuEl ?? null;
}
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

const modelMenuOpen = ref(false);
const modelMenuRevealed = ref(false);
const approvalMenuOpen = ref(false);
const approvalMenuRevealed = ref(false);
const askQuestionIndex = ref(0);
const askAnswers = ref<Record<number, string[]>>({});
const askUserFinishing = ref(false);

const settingStore = useSettingStore();
const chatModelStore = useChatModelStore();
const { language } = storeToRefs(settingStore);

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

// 每个窗口独立的本地模型选择，初始值从全局设置读取，但切换时不写入全局设置
const chatModel = ref(settingStore.chatModel);

const modelStatusText = computed(() => ({
  loading: tr(language.value, "loadingModels"),
  empty: tr(language.value, "noModels"),
}));

const availableModels = computed(() => {
  const models = [...chatModelStore.models];
  const current = chatModel.value.trim();

  if (current && !models.some((model) => model.id === current)) {
    models.unshift({ id: current, ownedBy: "" });
  }

  return models;
});

const modelBadgeTitle = computed(() => tr(language.value, "currentModel", { model: chatModel.value }));
const approvalModeOptions = computed(() =>
  toolApprovalModeOptions.map((option) => ({
    value: option.value,
    label: localizedOptionLabel(option, language.value),
  })),
);
const approvalModeLabel = computed(() => {
  const current = approvalModeOptions.value.find(
    (option) => option.value === settingStore.toolApprovalMode,
  );
  return current?.label ?? tr(language.value, "toolApprovalMode");
});
const approvalBadgeTitle = computed(() =>
  tr(language.value, "currentApprovalMode", { mode: approvalModeLabel.value }),
);

const showHistoryPicker = computed(() => props.historySessions !== null);

const historyItems = computed(() => props.historySessions ?? []);

const historyHeader = computed(() => tr(language.value, "chatHistory"));

const historyEmptyText = computed(() => tr(language.value, "noChats"));

const historyPickerRowCount = computed(() =>
  showHistoryPicker.value ? 2 + Math.max(historyItems.value.length, 1) : 0,
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
const hasAttachmentTags = computed(
  () =>
    Boolean(props.selectionLines) ||
    Boolean(pastedText.value) ||
    mentionedFiles.value.length > 0,
);

const inputPlaceholder = computed(() => {
  if (prefixText.value || hasAttachmentTags.value) {
    return "";
  }
  if (props.sending && !interactivePickerOpen.value) {
    return tr(language.value, "aiResponding");
  }
  if (showHistoryPicker.value) {
    return tr(language.value, "openChatHint");
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
  mentionedFiles.value.length > 0,
);

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

function emitLayoutChange() {
  const pickerRows = workspacePickerOpen.value
    ? workspacePickerRowCount.value
    : showAskUserPicker.value
    ? askPickerRowCount.value
    : showPathPermissionPicker.value
      ? pathPermissionPickerRowCount.value
      : showToolApprovalPicker.value
        ? toolApprovalPickerRowCount.value
        : showHistoryPicker.value
        ? historyPickerRowCount.value
        : showSuggestions.value
          ? suggestionCount.value
          : 0;

  emit("layoutChange", {
    showSuggestions: showSuggestions.value,
    suggestionCount: suggestionCount.value,
    showModelMenu: modelMenuOpen.value || approvalMenuOpen.value,
    modelMenuHeight:
      modelMenuOpen.value
        ? modelMenuHeight.value + MODEL_MENU_GAP
        : approvalMenuOpen.value
          ? approvalMenuHeight.value + MODEL_MENU_GAP
          : 0,
    askUserRowCount: showAskUserPicker.value ? askPickerRowCount.value : 0,
    pickerRowCount: pickerRows,
  });
}

async function syncPopupState(open: boolean) {
  const windowLabel = getCurrentWebviewWindow().label;
  try {
    await setOverlayPopupOpen(windowLabel, open);
  } catch (error) {
    console.error("set_overlay_popup_open failed:", error);
  }
}

async function updateModelMenuPosition() {
  await nextTick();

  const trigger = modelTriggerRef.value;
  const menu = getModelMenuEl();
  if (!trigger || !menu) {
    return;
  }

  const zoom = (settingStore.zoom || 100) / 100;
  const triggerRect = trigger.getBoundingClientRect();
  
  const unzoomedTriggerRect = {
    top: triggerRect.top / zoom,
    right: triggerRect.right / zoom,
    width: triggerRect.width / zoom,
  };

  const menuWidth = Math.max(unzoomedTriggerRect.width, MODEL_MENU_MIN_WIDTH);
  const measuredHeight = menu.offsetHeight;

  modelMenuHeight.value = measuredHeight;

  let left = unzoomedTriggerRect.right - menuWidth;
  left = Math.max(8, Math.min(left, window.innerWidth - menuWidth - 8));

  const top = Math.max(8, unzoomedTriggerRect.top - measuredHeight - MODEL_MENU_GAP);

  modelMenuStyle.value = {
    top: `${top}px`,
    left: `${left}px`,
    width: `${menuWidth}px`,
    visibility: modelMenuRevealed.value ? "visible" : "hidden",
    pointerEvents: modelMenuRevealed.value ? "auto" : "none",
  };

  emitLayoutChange();
}

function closeModelMenu(immediate = false) {
  if (!modelMenuOpen.value) {
    return;
  }

  const menu = getModelMenuEl();
  const finish = () => {
    modelMenuOpen.value = false;
    modelMenuRevealed.value = false;
    modelMenuHeight.value = 0;
    modelMenuStyle.value = {
      visibility: "hidden",
      pointerEvents: "none",
    };
    if (!approvalMenuOpen.value) {
      void syncPopupState(false);
    }
    emitLayoutChange();
  };

  if (immediate || !menu) {
    finish();
    return;
  }

  gsapMenuLeave(menu, finish);
}

async function updateApprovalMenuPosition() {
  await nextTick();

  const trigger = approvalTriggerRef.value;
  const menu = getApprovalMenuEl();
  if (!trigger || !menu) {
    return;
  }

  const zoom = (settingStore.zoom || 100) / 100;
  const triggerRect = trigger.getBoundingClientRect();
  const unzoomedTriggerRect = {
    top: triggerRect.top / zoom,
    right: triggerRect.right / zoom,
    width: triggerRect.width / zoom,
  };

  const menuWidth = Math.max(unzoomedTriggerRect.width, 140);
  const measuredHeight = menu.offsetHeight;
  approvalMenuHeight.value = measuredHeight;

  let left = unzoomedTriggerRect.right - menuWidth;
  left = Math.max(8, Math.min(left, window.innerWidth - menuWidth - 8));
  const top = Math.max(8, unzoomedTriggerRect.top - measuredHeight - MODEL_MENU_GAP);

  approvalMenuStyle.value = {
    top: `${top}px`,
    left: `${left}px`,
    width: `${menuWidth}px`,
    visibility: approvalMenuRevealed.value ? "visible" : "hidden",
    pointerEvents: approvalMenuRevealed.value ? "auto" : "none",
  };

  emitLayoutChange();
}

function closeApprovalMenu(immediate = false) {
  if (!approvalMenuOpen.value) {
    return;
  }

  const menu = getApprovalMenuEl();
  const finish = () => {
    approvalMenuOpen.value = false;
    approvalMenuRevealed.value = false;
    approvalMenuHeight.value = 0;
    approvalMenuStyle.value = {
      visibility: "hidden",
      pointerEvents: "none",
    };
    if (!modelMenuOpen.value) {
      void syncPopupState(false);
    }
    emitLayoutChange();
  };

  if (immediate || !menu) {
    finish();
    return;
  }

  gsapMenuLeave(menu, finish);
}

async function openModelMenu() {
  closeApprovalMenu(true);
  modelMenuRevealed.value = false;
  modelMenuOpen.value = true;
  modelMenuStyle.value = {
    visibility: "hidden",
    pointerEvents: "none",
  };
  await syncPopupState(true);
  // Position while hidden, then fade in — avoids jump/flash at 0,0.
  await updateModelMenuPosition();
  const menu = getModelMenuEl();
  if (menu) {
    // Lock opacity before the menu becomes hittable/visible to paint.
    gsapMenuPrepare(menu);
    modelMenuRevealed.value = true;
    modelMenuStyle.value = {
      ...modelMenuStyle.value,
      visibility: "visible",
      pointerEvents: "none",
    };
    gsapMenuEnter(menu, () => {
      modelMenuStyle.value = {
        ...modelMenuStyle.value,
        pointerEvents: "auto",
      };
    });
  }

  // Prefer cached list to avoid loading flash; refresh quietly in background.
  if (chatModelStore.models.length === 0) {
    void chatModelStore.fetch().then(() => {
      if (modelMenuOpen.value) {
        void updateModelMenuPosition();
      }
    });
  } else {
    void chatModelStore.softRefresh().then(() => {
      if (modelMenuOpen.value) {
        void updateModelMenuPosition();
      }
    });
  }
}

async function openApprovalMenu() {
  closeModelMenu(true);
  approvalMenuRevealed.value = false;
  approvalMenuOpen.value = true;
  approvalMenuStyle.value = {
    visibility: "hidden",
    pointerEvents: "none",
  };
  await syncPopupState(true);
  await updateApprovalMenuPosition();
  const menu = getApprovalMenuEl();
  if (menu) {
    gsapMenuPrepare(menu);
    approvalMenuRevealed.value = true;
    approvalMenuStyle.value = {
      ...approvalMenuStyle.value,
      visibility: "visible",
      pointerEvents: "none",
    };
    gsapMenuEnter(menu, () => {
      approvalMenuStyle.value = {
        ...approvalMenuStyle.value,
        pointerEvents: "auto",
      };
    });
  }
}

function toggleModelMenu() {
  if (modelMenuOpen.value) {
    closeModelMenu();
    return;
  }

  void openModelMenu();
}

function toggleApprovalMenu() {
  if (approvalMenuOpen.value) {
    closeApprovalMenu();
    return;
  }

  void openApprovalMenu();
}

function selectModel(modelId: string) {
  closeModelMenu();
  if (modelId === chatModel.value) {
    return;
  }

  // 只更新本地状态，不写入全局设置，避免所有窗口联动切换
  chatModel.value = modelId;
  emit("modelChange", modelId);
}

function selectApprovalMode(mode: ToolApprovalMode) {
  closeApprovalMenu();
  if (mode === settingStore.toolApprovalMode) {
    return;
  }
  void settingStore.update({ toolApprovalMode: mode });
}

onClickOutside(
  getModelMenuEl,
  () => {
    closeModelMenu();
  },
  { ignore: [modelTriggerRef] },
);

onClickOutside(
  getApprovalMenuEl,
  () => {
    closeApprovalMenu();
  },
  { ignore: [approvalTriggerRef] },
);

useEventListener(window, "resize", () => {
  if (modelMenuOpen.value) {
    void updateModelMenuPosition();
  }
  if (approvalMenuOpen.value) {
    void updateApprovalMenuPosition();
  }
});

useEventListener(window, "scroll", () => {
  if (modelMenuOpen.value) {
    void updateModelMenuPosition();
  }
  if (approvalMenuOpen.value) {
    void updateApprovalMenuPosition();
  }
}, { capture: true });

onMounted(async () => {
  void chatModelStore.fetch();
  await loadWorkspaceState();
  unlistenWorkspaces = await listen("workspaces-changed", () => {
    void loadWorkspaceState();
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
    message.value.startsWith("/") &&
    !message.value.includes(" "),
);

const filteredCommands = computed(() => {
  if (!isCommandMode.value) {
    return [];
  }

  const query = message.value.toLowerCase();
  return slashCommands.filter(
    (item) =>
      item.command.toLowerCase().startsWith(query) &&
      (item.command !== "/work" || props.showWorkspaceButton),
  );
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
  if (!interactivePickerOpen.value && !modelMenuOpen.value && !approvalMenuOpen.value) {
    return;
  }
  if (event.target === inputRef.value) {
    return;
  }
  handleKeydown(event);
}

function restorePickerFocus() {
  if (interactivePickerOpen.value || modelMenuOpen.value || approvalMenuOpen.value) {
    void focusInput();
  }
}

async function executeCommand(command: string) {
  message.value = "";
  prefixText.value = "";
  selectedIndex.value = 0;
  emitLayoutChange();
  const action = await executeSlashCommand(command);
  if (action === "openHistory") {
    emit("openHistory");
    return;
  }
  if (action === "openWorkspace") {
    await openWorkspaceQuickPicker();
    return;
  }
  if (action === "enterPlan") {
    emit("enterPlan");
    return;
  }
  if (action === "clearInput") {
    reset();
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
  closeModelMenu();
  closeApprovalMenu();
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
  closeModelMenu();
  closeApprovalMenu();
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
  if (props.sending && !interactivePickerOpen.value) {
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
  if (!text && !pastedText.value && mentionedFiles.value.length === 0) {
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

  if (props.enableCommands && slashCommands.some((item) => item.command === text)) {
    await executeCommand(text);
    return;
  }

  const fileMentions = mentionedFiles.value
    .map((path) => (/\s/.test(path) ? `@"${path}"` : `@${path}`))
    .join(" ");
  const submittedText = [text, fileMentions, pastedText.value]
    .filter((part) => part.length > 0)
    .join("\n\n");
  emit("submit", submittedText);
  message.value = "";
  prefixText.value = "";
  pastedText.value = "";
  mentionedFiles.value = [];
  emitLayoutChange();
}

function handlePaste(event: ClipboardEvent) {
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
  if (event.key === "Backspace" || event.key === "Delete") {
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
    // 回复中：允许输入，但回车不发送也不触发暂停（只能鼠标点暂停）
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

  if (modelMenuOpen.value) {
    if (event.key === "Escape") {
      event.preventDefault();
      closeModelMenu();
      return;
    }
  }

  if (approvalMenuOpen.value) {
    if (event.key === "Escape") {
      event.preventDefault();
      closeApprovalMenu();
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
  selectedIndex.value = 0;
  closeModelMenu();
  closeApprovalMenu();
  workspacePickerOpen.value = false;
  workspaceQuickSelectOnly.value = false;
  emitLayoutChange();
}

function setMessage(text: string) {
  prefixText.value = "";
  pastedText.value = "";
  mentionedFiles.value = [];
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
  [() => chatModelStore.loading, () => chatModelStore.error, availableModels],
  () => {
    if (modelMenuOpen.value) {
      void updateModelMenuPosition();
    }
  },
);

watch(showSuggestions, () => {
  if (showSuggestions.value && modelMenuOpen.value) {
    closeModelMenu();
  }
  if (showSuggestions.value && approvalMenuOpen.value) {
    closeApprovalMenu();
  }
  emitLayoutChange();
}, { immediate: true });

watch(showAskUserPicker, () => {
  if (showAskUserPicker.value && modelMenuOpen.value) {
    closeModelMenu();
  }
  if (showAskUserPicker.value && approvalMenuOpen.value) {
    closeApprovalMenu();
  }
  emitLayoutChange();
});

watch(
  () => props.pathPermission,
  async (session) => {
    if (session) {
      selectedIndex.value = 0;
      if (modelMenuOpen.value) {
        closeModelMenu();
      }
      if (approvalMenuOpen.value) {
        closeApprovalMenu();
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
      if (modelMenuOpen.value) {
        closeModelMenu();
      }
      if (approvalMenuOpen.value) {
        closeApprovalMenu();
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
      if (modelMenuOpen.value) {
        closeModelMenu();
      }
      if (approvalMenuOpen.value) {
        closeApprovalMenu();
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
  min-height: 26px;
  justify-content: space-between;
  gap: 10px;
}

.input-footer-primary,
.input-footer-actions {
  min-width: 0;
  gap: 6px;
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
}

.model-picker {
  flex: none;
  min-width: 0;
}

.workspace-control {
  position: relative;
  flex: none;
  height: 26px;
  max-width: 140px;
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
  min-width: 26px;
  max-width: 140px;
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
}

.workspace-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 11px;
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
  box-shadow: 0 2px 6px rgb(0 0 0 / 18%);
  cursor: pointer;
  opacity: 0;
  pointer-events: none;
  transform: scale(0.72);
  transition: background 120ms ease, border-color 120ms ease, color 120ms ease, opacity 120ms ease, transform 120ms ease;
}

.workspace-control:hover .workspace-exit-btn,
.workspace-control:focus-within .workspace-exit-btn {
  opacity: 1;
  pointer-events: auto;
  transform: scale(1);
}

.workspace-exit-btn:hover {
  border-color: #ef4444;
  background: #ef4444;
  color: white;
}

.model-badge {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  max-width: 190px;
  height: 26px;
  margin: 0;
  padding: 0 7px;
  border-radius: 6px;
  border: 1px solid var(--peek-border);
  background: color-mix(in srgb, var(--peek-accent) 8%, transparent);
  color: var(--peek-muted);
  font-family: var(--peek-font-sans);
  font-size: 11px;
  line-height: 1;
  user-select: none;
  cursor: pointer;
  appearance: none;
  transition: border-color 120ms ease, color 120ms ease, background 120ms ease;
}

.model-badge > svg {
  flex: none;
}

.model-name {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.model-chevron {
  flex: none;
  transition: transform 120ms ease;
}

.model-badge.open .model-chevron {
  transform: rotate(180deg);
}

.model-badge:hover,
.model-badge.open {
  border-color: color-mix(in srgb, var(--peek-accent) 35%, var(--peek-border));
  color: var(--peek-text);
  background: color-mix(in srgb, var(--peek-accent) 14%, transparent);
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
  transition: background 120ms ease, color 120ms ease;
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
  background: color-mix(in srgb, #f87171 18%, var(--peek-send-bg));
  color: color-mix(in srgb, #f87171 85%, var(--peek-send-fg));
  cursor: pointer;
}
</style>

<style>
.model-menu-floating {
  position: fixed;
  z-index: 10000;
  list-style: none;
  margin: 0;
  padding: 4px;
  border-radius: 10px;
  border: 1px solid var(--peek-border);
  background: var(--peek-list-bg);
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.22);
  max-height: 220px;
  overflow-y: auto;
  transform-origin: bottom right;
  will-change: opacity, transform;
}

.model-menu-floating .model-menu-item {
  display: flex;
  flex-direction: column;
  gap: 1px;
  padding: 7px 10px;
  min-height: 34px;
  justify-content: center;
  border-radius: 7px;
  cursor: default;
}

.model-menu-floating .model-menu-item:hover,
.model-menu-floating .model-menu-item.active {
  background: var(--peek-list-active);
}

.model-menu-floating .model-option-name {
  font-size: 12px;
  color: var(--peek-text);
}

.model-menu-floating .model-option-id {
  font-family: var(--font-mono);
  font-size: 10px;
  color: var(--peek-muted);
}

.model-menu-floating .model-status {
  font-size: 12px;
  color: var(--peek-muted);
}

.model-menu-floating .model-status.error {
  color: #e07a7a;
  line-height: 1.4;
}
</style>

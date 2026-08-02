<template>
  <main class="workbench" :data-theme="builtInTheme" @click="workspaceMenuId = ''">
    <AppConfirmDialog ref="confirmDialogRef" />
    <Transition name="workbench-ready">
      <WorkbenchLoading v-if="initializing" />
    </Transition>
    <header class="titlebar" data-tauri-drag-region>
      <button
        type="button"
        class="brand"
        :title="labels.toggleNavigation"
        data-tauri-drag-region="false"
        @click="navigationOpen = !navigationOpen"
      >
        <PanelLeft :size="15" />
        <strong>AAAi</strong>
      </button>

      <div class="titlebar-context" data-tauri-drag-region>
        <span>{{ activeTitle }}</span>
      </div>

      <nav class="view-actions" :aria-label="labels.views" data-tauri-drag-region="false">
        <button
          type="button"
          class="icon-button"
          :class="{ active: reviewOpen }"
          :title="labels.views"
          :aria-label="labels.views"
          @click="toggleReviewSidebar"
        >
          <PanelRight :size="15" />
          <span v-if="runningSubagentCount" class="status-dot" />
        </button>
        <button type="button" class="icon-button" :class="{ active: settingsOpen }" :title="labels.settings" :aria-label="labels.settings" @click="toggleSettings">
          <Settings :size="15" />
        </button>
      </nav>

      <div class="window-actions" data-tauri-drag-region="false">
        <button type="button" class="window-button" :title="labels.minimize" @click="minimizeWindow">
          <Minus :size="14" />
        </button>
        <button
          type="button"
          class="window-button"
          :title="tr(settingStore.language, isMaximized ? 'restoreWindow' : 'maximizeWindow')"
          :aria-label="tr(settingStore.language, isMaximized ? 'restoreWindow' : 'maximizeWindow')"
          @click="toggleMaximizeWindow"
        >
          <span v-if="isMaximized" class="windows-caption-icon" aria-hidden="true">&#xE923;</span>
          <span v-else class="windows-caption-icon" aria-hidden="true">&#xE922;</span>
        </button>
        <button type="button" class="window-button close" :title="labels.close" @click="hideWindow">
          <X :size="14" />
        </button>
      </div>
    </header>

    <div v-if="settingsOpen" class="embedded-settings">
      <SettingsPage embedded @back="closeSettings" />
    </div>

    <div v-else class="workspace-grid" :class="{ 'navigation-closed': !navigationOpen, 'review-open': reviewOpen }">
      <aside v-show="navigationOpen" class="navigation-pane">
        <button type="button" class="new-chat-button" @click.stop="createQuickConversation">
          <SquarePen :size="15" />
          <span>{{ labels.newChat }}</span>
        </button>

        <nav class="session-list peek-scrollbar" :aria-label="labels.conversations" @click.stop>
          <section
            v-for="workspaceSection in workspaceNavigationSections"
            :key="workspaceSection.id"
            class="navigation-section"
          >
            <header class="navigation-section-header">
              <button
                type="button"
                class="navigation-section-toggle"
                @click="toggleNavigationSection(workspaceSection.id)"
              >
                <ChevronRight
                  :size="13"
                  :class="{ expanded: !collapsedNavigationSections.has(workspaceSection.id) }"
                />
                <span>{{ workspaceSection.label }}</span>
                <small>{{ workspaceSection.items.length }}</small>
              </button>
              <button
                v-if="workspaceSection.id === 'workspaces'"
                type="button"
                class="section-action"
                :title="navigationLabels.addWorkspace"
                @click="addWorkspace"
              >
                <Plus :size="14" />
              </button>
            </header>

            <div
              v-show="!collapsedNavigationSections.has(workspaceSection.id)"
              class="navigation-section-body"
            >
              <section
                v-for="workspace in workspaceSection.items"
                :key="workspace.id"
                class="workspace-group"
                :data-workspace-id="workspace.id"
                :class="{
                  dragging: draggedWorkspaceId === workspace.id,
                  'drop-before': dragOverWorkspaceId === workspace.id && workspaceDropPosition === 'before',
                  'drop-after': dragOverWorkspaceId === workspace.id && workspaceDropPosition === 'after',
                }"
              >
                <div
                  class="workspace-row"
                  role="button"
                  tabindex="0"
                  :aria-expanded="!collapsedWorkspaceIds.has(workspace.id)"
                  :title="collapsedWorkspaceIds.has(workspace.id) ? navigationLabels.expandWorkspace : navigationLabels.collapseWorkspace"
                  @click="handleWorkspaceClick(workspace)"
                  @keydown.enter.self.prevent="handleWorkspaceClick(workspace)"
                  @keydown.space.self.prevent="handleWorkspaceClick(workspace)"
                  @pointerdown="startWorkspacePointerDrag($event, workspace)"
                >
                  <span
                    class="workspace-collapse"
                    aria-hidden="true"
                  />
                  <span class="workspace-group-header">
                    <Folder v-if="collapsedWorkspaceIds.has(workspace.id)" :size="14" />
                    <FolderOpen v-else :size="14" />
                    <span>{{ workspace.name }}</span>
                  </span>
                  <div class="workspace-actions">
                    <button type="button" :title="navigationLabels.more" @click.stop="toggleWorkspaceMenu(workspace.id)">
                      <Ellipsis :size="14" />
                    </button>
                    <button type="button" :title="navigationLabels.newWorkspaceChat" @click.stop="createWorkspaceConversation(workspace)">
                      <SquarePen :size="13" />
                    </button>
                  </div>
                </div>
                <div v-if="workspaceMenuId === workspace.id" class="workspace-menu" @click.stop>
                  <button type="button" @click.stop="toggleWorkspacePinned(workspace)">
                    <PinOff v-if="workspace.pinned" :size="13" />
                    <Pin v-else :size="13" />
                    <span>{{ workspace.pinned ? navigationLabels.unpinWorkspace : navigationLabels.pinWorkspace }}</span>
                  </button>
                  <button type="button" @click.stop="openWorkspaceFolder(workspace)">
                    <FolderOpen :size="13" />
                    <span>{{ navigationLabels.openFolder }}</span>
                  </button>
                  <button type="button" class="danger" @click.stop="removeWorkspace(workspace)">
                    <Trash2 :size="13" />
                    <span>{{ navigationLabels.deleteWorkspace }}</span>
                  </button>
                </div>
                <WorkbenchSessionList
                  v-show="!collapsedWorkspaceIds.has(workspace.id)"
                  :sessions="sessionsForWorkspace(workspace.id)"
                  :active-session-id="activeSessionId"
                  :language="settingStore.language"
                  :untitled-label="labels.untitled"
                  :delete-label="labels.deleteConversation"
                  :running-session-ids="runningSessionIds"
                  :attention-session-ids="attentionSessionIds"
                  :unread-session-ids="unreadSessionIdList"
                  variant="workspace"
                  @select="selectConversation"
                  @delete="removeConversation"
                />
              </section>
            </div>
          </section>

          <section class="navigation-section quick-ask-section">
            <header class="navigation-section-header">
              <button type="button" class="navigation-section-toggle" @click="toggleNavigationSection('quick')">
                <ChevronRight :size="13" :class="{ expanded: !collapsedNavigationSections.has('quick') }" />
                <span>{{ navigationLabels.quickAsk }}</span>
                <small>{{ quickAskSessions.length }}</small>
              </button>
              <button type="button" class="section-action" :title="navigationLabels.newQuickAsk" @click="createQuickConversation">
                <SquarePen :size="13" />
              </button>
            </header>
            <WorkbenchSessionList
              v-show="!collapsedNavigationSections.has('quick')"
              :sessions="quickAskSessions"
              :active-session-id="activeSessionId"
              :language="settingStore.language"
              :untitled-label="labels.untitled"
              :delete-label="labels.deleteConversation"
              :running-session-ids="runningSessionIds"
              :attention-session-ids="attentionSessionIds"
              :unread-session-ids="unreadSessionIdList"
              variant="quick"
              @select="selectConversation"
              @delete="removeConversation"
            />
          </section>
        </nav>
      </aside>

      <section
        class="conversation-pane"
        :class="{ 'empty-conversation': !hasConversationMessages }"
      >
        <div v-if="contextNotice" class="context-notice">{{ contextNotice }}</div>
        <div v-if="!hasConversationMessages" class="empty-conversation-brand" aria-hidden="true">
          <img :src="appIconAsset" alt="" draggable="false" />
        </div>
        <p v-if="!hasConversationMessages" class="empty-conversation-prompt">
          {{ emptyConversationPrompt }}
        </p>
        <MessageList
          class="workbench-messages"
          :messages="messages"
          :session-id="activeSessionId"
          :checkpoints="checkpoints"
          @rewound="handleRewound"
          @review-changes="openReview('diff')"
          @inspect-subagent="openAgentReview"
          @preview-image="previewImage"
        />

        <div class="composer-wrap">
          <ChatInputBar
            ref="inputRef"
            :sending="sending"
            :close-on-escape="false"
            appearance="workbench"
            overlay-pickers
            :session-id="activeSessionId"
            :ask-user="askUserSession"
            :path-permission="pathPermissionSession"
            :tool-approval="toolApprovalSession"
            @submit="submitMessage"
            @pause="pauseResponse"
            @ask-user-complete="completeAskUser"
            @path-permission-complete="completePathPermission"
            @tool-approval-complete="completeToolApproval"
            @preview-image="previewImage"
          />
        </div>
      </section>

      <aside v-if="reviewOpen" class="review-pane">
        <header class="review-header">
          <div class="review-tabs" role="tablist" :aria-label="labels.views">
            <button
              v-for="view in reviewViews"
              :key="view.id"
              type="button"
              :class="{ active: reviewView === view.id }"
              @click="reviewView = view.id"
            >
              <component :is="view.icon" :size="14" />
              <span>{{ view.label }}</span>
            </button>
          </div>
          <button type="button" class="small-icon-button" :title="labels.closePanel" @click="reviewOpen = false">
            <PanelRightClose :size="15" />
          </button>
        </header>
        <CodeDiffSidebar v-show="reviewView === 'diff'" embedded :messages="messages" :width="reviewWidth" />
        <SubagentSidebar
          v-show="reviewView === 'agents'"
          embedded
          :activities="subagentActivities"
          :all-activities="allToolActivities"
          :opened-entry-ids="openedSubagentIds"
          :selected-entry-id="selectedSubagentId"
          @close-entry="closeSubagent"
        />
        <AgentDebugPanel v-show="reviewView === 'runtime'" embedded />
        <ImagePreviewSidebar
          v-show="reviewView === 'image'"
          :sources="openedImageSources"
          :selected-source="selectedImageSource"
          @select="selectedImageSource = $event"
          @close="closeImageTab"
        />
      </aside>
    </div>
  </main>
</template>

<script setup lang="ts">
import { computed, defineAsyncComponent, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  Bug,
  ChevronRight,
  Ellipsis,
  FileDiff,
  Folder,
  FolderOpen,
  Image as ImageIcon,
  Minus,
  PanelLeft,
  PanelRight,
  PanelRightClose,
  Pin,
  PinOff,
  Plus,
  Settings,
  SquarePen,
  Trash2,
  Workflow,
  X,
} from "@lucide/vue";
import AgentDebugPanel from "@/components/chat/AgentDebugPanel.vue";
import ChatInputBar, {
  type AskUserSession,
  type PathPermissionSession,
} from "@/components/chat/ChatInputBar.vue";
import CodeDiffSidebar from "@/components/chat/CodeDiffSidebar.vue";
import ImagePreviewSidebar from "@/components/chat/ImagePreviewSidebar.vue";
import MessageList from "@/components/chat/MessageList.vue";
import SubagentSidebar from "@/components/chat/SubagentSidebar.vue";
import WorkbenchSessionList from "@/components/workbench/WorkbenchSessionList.vue";
import WorkbenchLoading from "@/components/workbench/WorkbenchLoading.vue";
import { AppConfirmDialog } from "@/components/ui/confirm-dialog";
import appIconAsset from "../../src-tauri/icons/AAAi-transparent.svg";
import {
  chatCancel,
  deleteChatSession,
  listChatSessions,
  listCheckpoints,
  listenAskUser,
  listenChatFinished,
  listenInteractionResolved,
  listenPathPermission,
  listenToolApproval,
  respondAskUser,
  respondPathPermission,
  respondToolApproval,
  setWindowSessionView,
  showInteractionNotification,
} from "@/services/ipc";
import { tr } from "@/services/i18n";
import { estimateMessageTokens } from "@/services/chat/tokenEstimate";
import { useChatStore } from "@/stores/chat";
import { useSettingStore } from "@/stores/setting";
import {
  clearCurrentWorkspace,
  createWorkspace,
  deleteWorkspace,
  listWorkspaces,
  openWorkspaceFolder as openWorkspaceFolderCommand,
  reorderWorkspaces,
  selectWorkspaceFolder,
  setWorkspacePinned,
  switchWorkspace,
  type Workspace,
} from "@/commands/workspace";
import type {
  ChatSessionSummary,
  CheckpointInfo,
  PathPermissionDecision,
  ToolApprovalDecision,
  ToolApprovalSession,
} from "@/types/chat";

type ReviewView = "diff" | "agents" | "runtime" | "image";
type WorkspaceDropPosition = "before" | "after";
type PendingInteraction =
  | { kind: "ask_user"; value: AskUserSession }
  | { kind: "path_permission"; value: PathPermissionSession }
  | { kind: "tool_approval"; value: ToolApprovalSession };
type WorkspacePointerDrag = {
  pointerId: number;
  sourceId: string;
  startX: number;
  startY: number;
  dragging: boolean;
  cancelled: boolean;
  longPressTimer: number;
};

const SUBAGENT_TOOLS = new Set([
  "run_subagent",
  "run_readonly_subagent",
  "run_parallel_subagents",
  "run_skill",
  "run_readonly_skill",
  "explore_codebase",
  "research_topic",
  "review_code",
  "review_security",
  "generate_word",
]);

const SettingsPage = defineAsyncComponent(() => import("@/pages/Settings/index.vue"));

const chatStore = useChatStore();
const settingStore = useSettingStore();
const appWindow = getCurrentWebviewWindow();
const inputRef = ref<InstanceType<typeof ChatInputBar> | null>(null);
const confirmDialogRef = ref<InstanceType<typeof AppConfirmDialog> | null>(null);
const navigationOpen = ref(true);
const isMaximized = ref(false);
const settingsOpen = ref(false);
const builtInTheme = computed(() => settingStore.colorScheme);
const reviewOpen = ref(false);
const reviewView = ref<ReviewView>("diff");
const reviewWidth = ref(640);
const sessions = ref<ChatSessionSummary[]>([]);
const workspaces = ref<Workspace[]>([]);
const collapsedWorkspaceIds = ref(new Set<string>());
const collapsedNavigationSections = ref(new Set<string>());
const workspaceMenuId = ref("");
const draggedWorkspaceId = ref("");
const dragOverWorkspaceId = ref("");
const workspaceDropPosition = ref<WorkspaceDropPosition | null>(null);
const workspacePointerDrag = ref<WorkspacePointerDrag | null>(null);
const suppressedWorkspaceClickId = ref("");
const sessionsLoading = ref(false);
const activeSessionId = ref("");
const activeSessionWorkspaceId = ref<string | null>(null);
const initializing = ref(true);
const checkpoints = ref<CheckpointInfo[]>([]);
const pendingInteractions = ref<Record<string, PendingInteraction>>({});
const unreadSessionIds = ref(new Set<string>());
const openedSubagentIds = ref<string[]>([]);
const selectedSubagentId = ref("");
const openedImageSources = ref<string[]>([]);
const selectedImageSource = ref("");
const unlisteners: UnlistenFn[] = [];
let pendingWorkbenchSessionId = "";

const isChinese = computed(() => settingStore.language === "zh-CN");
const labels = computed(() => isChinese.value ? {
  toggleNavigation: "切换会话栏",
  views: "工作区视图",
  settings: "设置",
  minimize: "最小化",
  close: "关闭",
  newChat: "新对话",
  conversations: "对话",
  refresh: "刷新会话",
  untitled: "新对话",
  deleteConversation: "删除对话",
  noConversations: "还没有对话",
  closePanel: "关闭审阅区",
  deleteConfirm: "确定删除这个对话吗？此操作无法撤销。",
  diff: "差异",
  agents: "子 Agent",
  runtime: "运行时",
} : {
  toggleNavigation: "Toggle conversations",
  views: "Workspace views",
  settings: "Settings",
  minimize: "Minimize",
  close: "Close",
  newChat: "New chat",
  conversations: "Conversations",
  refresh: "Refresh conversations",
  untitled: "New conversation",
  deleteConversation: "Delete conversation",
  noConversations: "No conversations yet",
  closePanel: "Close review pane",
  deleteConfirm: "Delete this conversation? This cannot be undone.",
  diff: "Diff",
  agents: "Sub-agents",
  runtime: "Runtime",
});

const navigationLabels = computed(() => isChinese.value ? {
  pinned: "\u7f6e\u9876",
  workspaces: "\u5de5\u4f5c\u533a",
  quickAsk: "\u968f\u95ee",
  addWorkspace: "\u6dfb\u52a0\u5de5\u4f5c\u533a",
  pinWorkspace: "\u7f6e\u9876\u5de5\u4f5c\u533a",
  unpinWorkspace: "\u53d6\u6d88\u7f6e\u9876",
  newWorkspaceChat: "\u5728\u5de5\u4f5c\u533a\u65b0\u5efa\u4f1a\u8bdd",
  expandWorkspace: "\u5c55\u5f00\u5de5\u4f5c\u533a",
  collapseWorkspace: "\u6298\u53e0\u5de5\u4f5c\u533a",
  newQuickAsk: "\u65b0\u5efa\u968f\u95ee\u4f1a\u8bdd",
  more: "\u66f4\u591a\u9009\u9879",
  openFolder: "\u6253\u5f00\u6587\u4ef6\u5939",
  deleteWorkspace: "\u5220\u9664\u5de5\u4f5c\u533a",
  deleteWorkspaceConfirm: "\u5220\u9664\u8fd9\u4e2a\u5de5\u4f5c\u533a\uff1f\u5bf9\u8bdd\u4e0d\u4f1a\u88ab\u5220\u9664\u3002",
  cancel: "\u53d6\u6d88",
  confirmDelete: "\u5220\u9664",
} : {
  pinned: "Pinned",
  workspaces: "Workspaces",
  quickAsk: "Quick Ask",
  addWorkspace: "Add workspace",
  pinWorkspace: "Pin workspace",
  unpinWorkspace: "Unpin workspace",
  newWorkspaceChat: "New chat in workspace",
  expandWorkspace: "Expand workspace",
  collapseWorkspace: "Collapse workspace",
  newQuickAsk: "New quick ask",
  more: "More options",
  openFolder: "Open folder",
  deleteWorkspace: "Delete workspace",
  deleteWorkspaceConfirm: "Delete this workspace? Conversations will be kept.",
  cancel: "Cancel",
  confirmDelete: "Delete",
});

const activeWorkspaceName = computed(() =>
  workspaces.value.find((workspace) => workspace.id === activeSessionWorkspaceId.value)?.name,
);
const emptyConversationPrompt = computed(() => {
  if (!activeWorkspaceName.value) {
    return isChinese.value ? "我能为您做什么？" : "What can I do for you?";
  }
  return isChinese.value
    ? `需要我在 ${activeWorkspaceName.value} 中帮助您完成什么？`
    : `What would you like me to help you accomplish in ${activeWorkspaceName.value}?`;
});

const reviewViews = computed(() => [
  { id: "diff" as const, label: labels.value.diff, icon: FileDiff },
  { id: "agents" as const, label: labels.value.agents, icon: Workflow },
  ...(openedImageSources.value.length
    ? [{ id: "image" as const, label: tr(settingStore.language, "image.preview"), icon: ImageIcon }]
    : []),
  { id: "runtime" as const, label: labels.value.runtime, icon: Bug },
]);
const pinnedWorkspaces = computed(() => workspaces.value.filter((workspace) => workspace.pinned));
const regularWorkspaces = computed(() => workspaces.value.filter((workspace) => !workspace.pinned));
const workspaceNavigationSections = computed(() => [
  ...(pinnedWorkspaces.value.length > 0
    ? [{ id: "pinned", label: navigationLabels.value.pinned, items: pinnedWorkspaces.value }]
    : []),
  { id: "workspaces", label: navigationLabels.value.workspaces, items: regularWorkspaces.value },
]);
const messages = computed(() => chatStore.sessions[activeSessionId.value] ?? []);
const sessionsWithLiveTokens = computed(() => sessions.value.map((session) => {
  if (session.sessionId !== activeSessionId.value) return session;
  return {
    ...session,
    estimatedTokens: messages.value.reduce(
      (total, message) => total + estimateMessageTokens(message),
      0,
    ),
  };
}));
const quickAskSessions = computed(() => sessionsWithLiveTokens.value.filter((session) => !session.workspaceId));
const sessionsByWorkspace = computed(() => {
  const result = new Map<string, ChatSessionSummary[]>();
  for (const session of sessionsWithLiveTokens.value) {
    if (!session.workspaceId) continue;
    const items = result.get(session.workspaceId) ?? [];
    items.push(session);
    result.set(session.workspaceId, items);
  }
  return result;
});
const hasConversationMessages = computed(() =>
  messages.value.some((message) => message.role === "user" || message.role === "assistant"),
);
const sending = computed(() => Boolean(chatStore.sending[activeSessionId.value]));
const runningSessionIds = computed(() => Object.keys(chatStore.sending));
const attentionSessionIds = computed(() => Object.keys(pendingInteractions.value));
const unreadSessionIdList = computed(() => [...unreadSessionIds.value]);
const activePendingInteraction = computed(() => pendingInteractions.value[activeSessionId.value]);
const askUserSession = computed<AskUserSession | null>(() =>
  activePendingInteraction.value?.kind === "ask_user" ? activePendingInteraction.value.value : null,
);
const pathPermissionSession = computed<PathPermissionSession | null>(() =>
  activePendingInteraction.value?.kind === "path_permission" ? activePendingInteraction.value.value : null,
);
const toolApprovalSession = computed<ToolApprovalSession | null>(() =>
  activePendingInteraction.value?.kind === "tool_approval" ? activePendingInteraction.value.value : null,
);
const contextNotice = computed(() => chatStore.contextNotices[activeSessionId.value] ?? "");
const activeTitle = computed(() => settingsOpen.value
  ? labels.value.settings
  : sessions.value.find((session) => session.sessionId === activeSessionId.value)?.preview
    || labels.value.untitled,
);
const allToolActivities = computed(() => messages.value.flatMap((message) => message.toolActivities ?? []));
const subagentActivities = computed(() =>
  allToolActivities.value.filter((activity) => SUBAGENT_TOOLS.has(activity.toolName)),
);
const runningSubagentCount = computed(() =>
  subagentActivities.value.filter((activity) => activity.status === "running").length,
);
const activeAssistantMessageId = computed(() => [...messages.value].reverse().find(
  (message) => String(message.role).toLowerCase() === "assistant"
    && (message.status === "pending" || message.status === "streaming"),
)?.id ?? "");

function createSessionId() {
  return `session-${Date.now()}`;
}

function sessionsForWorkspace(workspaceId: string) {
  return sessionsByWorkspace.value.get(workspaceId) ?? [];
}

function setPendingInteraction(sessionId: string, interaction: PendingInteraction) {
  if (!sessionId) return;
  pendingInteractions.value = { ...pendingInteractions.value, [sessionId]: interaction };
}

function removePendingInteraction(sessionId: string, requestId?: string) {
  const current = pendingInteractions.value[sessionId];
  if (!current || (requestId && current.value.requestId !== requestId)) return false;
  const next = { ...pendingInteractions.value };
  delete next[sessionId];
  pendingInteractions.value = next;
  return true;
}

function markSessionUnread(sessionId: string) {
  if (!sessionId) return;
  unreadSessionIds.value = new Set([...unreadSessionIds.value, sessionId]);
}

function clearSessionUnread(sessionId: string) {
  if (!unreadSessionIds.value.has(sessionId)) return;
  const next = new Set(unreadSessionIds.value);
  next.delete(sessionId);
  unreadSessionIds.value = next;
}

async function isSessionBeingViewed(sessionId: string) {
  if (
    sessionId !== activeSessionId.value
    || settingsOpen.value
    || document.visibilityState !== "visible"
  ) return false;
  return (await appWindow.isVisible()) && (await appWindow.isFocused());
}

function sessionDisplayName(sessionId: string) {
  return sessions.value.find((session) => session.sessionId === sessionId)?.preview || labels.value.untitled;
}

async function showActionableWindowsNotification(
  sessionId: string,
  title: string,
  body: string,
  persistent = false,
) {
  await showInteractionNotification({
    sessionId,
    title,
    body,
    ignoreLabel: tr(settingStore.language, "notification.ignore"),
    openLabel: tr(settingStore.language, "notification.openConversation"),
    persistent,
  });
}

async function notifyWhenNotViewed(sessionId: string, title: string, body: string) {
  if (await isSessionBeingViewed(sessionId)) return false;
  await showActionableWindowsNotification(sessionId, title, body, true);
  return true;
}

function toggleNavigationSection(id: string) {
  const next = new Set(collapsedNavigationSections.value);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  collapsedNavigationSections.value = next;
}

function toggleWorkspaceMenu(id: string) {
  workspaceMenuId.value = workspaceMenuId.value === id ? "" : id;
}

function handleWorkspaceClick(workspace: Workspace) {
  if (suppressedWorkspaceClickId.value === workspace.id) {
    suppressedWorkspaceClickId.value = "";
    return;
  }
  toggleWorkspaceGroup(workspace.id);
}

function startWorkspacePointerDrag(event: PointerEvent, workspace: Workspace) {
  if (event.button !== 0 || !(event.target instanceof Element)) return;
  if (event.target.closest(".workspace-actions, .workspace-menu")) return;

  if (event.currentTarget instanceof HTMLElement) {
    event.currentTarget.setPointerCapture(event.pointerId);
  }
  const drag: WorkspacePointerDrag = {
    pointerId: event.pointerId,
    sourceId: workspace.id,
    startX: event.clientX,
    startY: event.clientY,
    dragging: false,
    cancelled: false,
    longPressTimer: 0,
  };
  drag.longPressTimer = globalThis.setTimeout(() => {
    const current = workspacePointerDrag.value;
    if (!current || current.pointerId !== drag.pointerId || current.sourceId !== drag.sourceId) return;
    current.dragging = true;
    draggedWorkspaceId.value = current.sourceId;
    workspaceMenuId.value = "";
    document.getSelection()?.removeAllRanges();
  }, 260);
  workspacePointerDrag.value = drag;
}

function moveWorkspacePointerDrag(event: PointerEvent) {
  const drag = workspacePointerDrag.value;
  if (!drag || drag.pointerId !== event.pointerId) return;
  if (drag.cancelled) return;

  if (!drag.dragging) {
    const distance = Math.hypot(event.clientX - drag.startX, event.clientY - drag.startY);
    if (distance < 6) return;
    clearWorkspaceLongPress(drag);
    drag.cancelled = true;
    return;
  }

  event.preventDefault();
  const targetElement = document
    .elementFromPoint(event.clientX, event.clientY)
    ?.closest<HTMLElement>("[data-workspace-id]");
  const targetId = targetElement?.dataset.workspaceId ?? "";
  const source = workspaces.value.find((workspace) => workspace.id === drag.sourceId);
  const target = workspaces.value.find((workspace) => workspace.id === targetId);
  const validTarget = source && target && source.id !== target.id && source.pinned === target.pinned;
  dragOverWorkspaceId.value = validTarget ? target.id : "";
  if (validTarget && targetElement) {
    const targetRow = targetElement.querySelector<HTMLElement>(":scope > .workspace-row");
    const bounds = targetRow?.getBoundingClientRect();
    workspaceDropPosition.value = bounds && event.clientY >= bounds.top + bounds.height / 2
      ? "after"
      : "before";
  } else {
    workspaceDropPosition.value = null;
  }
}

function finishWorkspacePointerDrag(event: PointerEvent) {
  const drag = workspacePointerDrag.value;
  if (!drag || drag.pointerId !== event.pointerId) return;

  const targetId = dragOverWorkspaceId.value;
  const dropPosition = workspaceDropPosition.value;
  clearWorkspaceLongPress(drag);
  workspacePointerDrag.value = null;
  draggedWorkspaceId.value = "";
  dragOverWorkspaceId.value = "";
  workspaceDropPosition.value = null;
  if (!drag.dragging) {
    if (drag.cancelled) suppressWorkspaceClick(drag.sourceId);
    return;
  }

  event.preventDefault();
  suppressWorkspaceClick(drag.sourceId);
  if (targetId && dropPosition) void reorderWorkspaceItems(drag.sourceId, targetId, dropPosition);
}

function cancelWorkspacePointerDrag(event: PointerEvent) {
  const drag = workspacePointerDrag.value;
  if (!drag || drag.pointerId !== event.pointerId) return;
  clearWorkspaceLongPress(drag);
  workspacePointerDrag.value = null;
  draggedWorkspaceId.value = "";
  dragOverWorkspaceId.value = "";
  workspaceDropPosition.value = null;
}

function clearWorkspaceLongPress(drag: WorkspacePointerDrag) {
  if (drag.longPressTimer) globalThis.clearTimeout(drag.longPressTimer);
  drag.longPressTimer = 0;
}

function suppressWorkspaceClick(workspaceId: string) {
  suppressedWorkspaceClickId.value = workspaceId;
  globalThis.setTimeout(() => {
    if (suppressedWorkspaceClickId.value === workspaceId) suppressedWorkspaceClickId.value = "";
  }, 300);
}

async function reorderWorkspaceItems(
  sourceId: string,
  targetId: string,
  dropPosition: WorkspaceDropPosition,
) {
  if (!sourceId || sourceId === targetId) return;
  const sourceIndex = workspaces.value.findIndex((workspace) => workspace.id === sourceId);
  const targetIndex = workspaces.value.findIndex((workspace) => workspace.id === targetId);
  const source = workspaces.value[sourceIndex];
  const target = workspaces.value[targetIndex];
  if (!source || !target || source.pinned !== target.pinned) return;

  const next = workspaces.value.filter((workspace) => workspace.id !== sourceId);
  const adjustedTargetIndex = next.findIndex((workspace) => workspace.id === targetId);
  const insertionIndex = adjustedTargetIndex + (dropPosition === "after" ? 1 : 0);
  next.splice(insertionIndex, 0, source);
  workspaces.value = next;
  try {
    await reorderWorkspaces(next.map((workspace) => workspace.id));
  } catch (error) {
    console.error("reorder workspaces failed:", error);
    workspaces.value = await listWorkspaces();
  }
}

async function toggleWorkspacePinned(workspace: Workspace) {
  workspaceMenuId.value = "";
  try {
    await setWorkspacePinned(workspace.id, !workspace.pinned);
    workspaces.value = await listWorkspaces();
  } catch (error) {
    console.error("set workspace pinned failed:", error);
  }
}

async function addWorkspace() {
  const root = await selectWorkspaceFolder();
  if (!root) return;
  await createWorkspace(root);
  await refreshSessions();
}

async function createWorkspaceConversation(workspace: Workspace) {
  await switchWorkspace(workspace.id);
  createConversation(workspace.id);
  await refreshSessions();
}

async function createQuickConversation() {
  await clearCurrentWorkspace();
  createConversation(null);
}

async function openWorkspaceFolder(workspace: Workspace) {
  workspaceMenuId.value = "";
  try {
    await openWorkspaceFolderCommand(workspace.id);
  } catch (error) {
    console.error("open workspace folder failed:", error);
  }
}

async function removeWorkspace(workspace: Workspace) {
  workspaceMenuId.value = "";
  const confirmed = await confirmDialogRef.value?.ask({
    title: navigationLabels.value.deleteWorkspace,
    description: navigationLabels.value.deleteWorkspaceConfirm,
    confirmLabel: navigationLabels.value.confirmDelete,
    cancelLabel: navigationLabels.value.cancel,
  });
  if (!confirmed) return;
  await deleteWorkspace(workspace.id);
  if (activeSessionWorkspaceId.value === workspace.id) {
    await clearCurrentWorkspace();
    activeSessionWorkspaceId.value = null;
  }
  await refreshSessions();
}

function createConversation(workspaceId: string | null) {
  const sessionId = createSessionId();
  chatStore.setSessionMessages(sessionId, []);
  chatStore.setOverlayDraftSession(sessionId);
  activeSessionId.value = sessionId;
  activeSessionWorkspaceId.value = workspaceId;
  checkpoints.value = [];
  reviewOpen.value = false;
  void nextTick(() => inputRef.value?.focusInput());
}

async function refreshSessions() {
  sessionsLoading.value = true;
  try {
    const [chatResponse, workspaceResponse] = await Promise.all([
      listChatSessions(),
      listWorkspaces().catch(() => []),
    ]);
    sessions.value = chatResponse.sessions;
    workspaces.value = workspaceResponse;
  } catch (error) {
    console.error("list_chat_sessions failed:", error);
  } finally {
    sessionsLoading.value = false;
  }
}

function toggleWorkspaceGroup(id: string) {
  const next = new Set(collapsedWorkspaceIds.value);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  collapsedWorkspaceIds.value = next;
}

async function selectConversation(sessionId: string) {
  clearSessionUnread(sessionId);
  const summary = sessions.value.find((session) => session.sessionId === sessionId);
  activeSessionWorkspaceId.value = summary?.workspaceId ?? null;
  if (summary?.workspaceId) {
    await switchWorkspace(summary.workspaceId);
  } else {
    await clearCurrentWorkspace();
  }
  activeSessionId.value = sessionId;
  chatStore.setOverlayDraftSession(sessionId);
  await chatStore.loadHistory(sessionId);
  await refreshCheckpoints();
  void nextTick(() => inputRef.value?.focusInput());
}

async function removeConversation(sessionId: string) {
  const confirmed = await confirmDialogRef.value?.ask({
    title: labels.value.deleteConversation,
    description: labels.value.deleteConfirm,
    confirmLabel: navigationLabels.value.confirmDelete,
    cancelLabel: navigationLabels.value.cancel,
  });
  if (!confirmed) return;
  await deleteChatSession(sessionId);
  delete chatStore.sessions[sessionId];
  clearSessionUnread(sessionId);
  removePendingInteraction(sessionId);
  await refreshSessions();
  if (activeSessionId.value === sessionId) {
    const next = sessions.value[0]?.sessionId;
    if (next) await selectConversation(next);
    else await createQuickConversation();
  }
}

async function submitMessage(text: string) {
  if (!text.trim()) return;
  if (!activeSessionId.value) await createQuickConversation();
  await chatStore.send(text, activeSessionId.value, {
    workspaceId: activeSessionWorkspaceId.value ?? undefined,
    quickAsk: !activeSessionWorkspaceId.value,
  });
  await refreshSessions();
  await refreshCheckpoints();
}

async function pauseResponse() {
  const messageId = activeAssistantMessageId.value;
  if (!messageId) return;
  chatStore.clearSending(activeSessionId.value);
  try {
    await chatCancel({ messageId });
  } catch (error) {
    console.error("chat_cancel failed:", error);
    chatStore.settleInterruptedSession(activeSessionId.value);
  }
}

async function refreshCheckpoints() {
  if (!activeSessionId.value) {
    checkpoints.value = [];
    return;
  }
  try {
    checkpoints.value = await listCheckpoints(activeSessionId.value);
  } catch {
    checkpoints.value = [];
  }
}

async function handleRewound(payload: { text: string }) {
  await chatStore.loadHistory(activeSessionId.value);
  await refreshCheckpoints();
  if (payload.text) inputRef.value?.setMessage(payload.text);
}

async function completeAskUser(answer: string) {
  const session = askUserSession.value;
  if (!session) return;
  const sessionId = activeSessionId.value;
  removePendingInteraction(sessionId, session.requestId);
  try {
    await respondAskUser({ requestId: session.requestId, answer });
    chatStore.completeAskUserToolActivities(sessionId, answer);
  } catch (error) {
    if (!isAlreadyResolvedError(error) && !pendingInteractions.value[sessionId]) {
      setPendingInteraction(sessionId, { kind: "ask_user", value: session });
    }
    console.error("respond_ask_user failed:", error);
  }
}

async function completePathPermission(decision: PathPermissionDecision) {
  const session = pathPermissionSession.value;
  if (!session) return;
  const sessionId = activeSessionId.value;
  removePendingInteraction(sessionId, session.requestId);
  try {
    await respondPathPermission({ requestId: session.requestId, decision });
  } catch (error) {
    if (!isAlreadyResolvedError(error) && !pendingInteractions.value[sessionId]) {
      setPendingInteraction(sessionId, { kind: "path_permission", value: session });
    }
    console.error("respond_path_permission failed:", error);
  }
}

async function completeToolApproval(decision: ToolApprovalDecision) {
  const session = toolApprovalSession.value;
  if (!session) return;
  const sessionId = activeSessionId.value;
  removePendingInteraction(sessionId, session.requestId);
  try {
    await respondToolApproval({ requestId: session.requestId, decision });
  } catch (error) {
    if (!isAlreadyResolvedError(error) && !pendingInteractions.value[sessionId]) {
      setPendingInteraction(sessionId, { kind: "tool_approval", value: session });
    }
    console.error("respond_tool_approval failed:", error);
  }
}

function isAlreadyResolvedError(error: unknown) {
  return String(error).includes("already completed") || String(error).includes("not found");
}

function openReview(view: ReviewView) {
  reviewView.value = view;
  reviewOpen.value = true;
}

function toggleReviewSidebar() {
  if (reviewOpen.value) {
    reviewOpen.value = false;
    return;
  }

  if (reviewView.value === "agents" && !openedSubagentIds.value.length) {
    reviewView.value = "diff";
  }
  reviewOpen.value = true;
}

function openAgentReview(activityId: string) {
  if (!openedSubagentIds.value.includes(activityId)) openedSubagentIds.value.push(activityId);
  selectedSubagentId.value = activityId;
  openReview("agents");
}

function closeSubagent(activityId: string) {
  openedSubagentIds.value = openedSubagentIds.value.filter((id) => id !== activityId);
  if (selectedSubagentId.value === activityId) {
    selectedSubagentId.value = openedSubagentIds.value[openedSubagentIds.value.length - 1] ?? "";
  }
}

function previewImage(source: string) {
  if (!openedImageSources.value.includes(source)) {
    openedImageSources.value = [...openedImageSources.value, source];
  }
  selectedImageSource.value = source;
  openReview("image");
}

function closeImageTab(source: string) {
  const index = openedImageSources.value.indexOf(source);
  if (index < 0) return;

  const remaining = openedImageSources.value.filter((item) => item !== source);
  openedImageSources.value = remaining;
  if (selectedImageSource.value === source) {
    selectedImageSource.value = remaining[index] ?? remaining[index - 1] ?? "";
  }
  if (!remaining.length && reviewView.value === "image") {
    reviewView.value = "diff";
  }
}

function minimizeWindow() {
  void appWindow.minimize();
}

async function syncMaximizedState() {
  isMaximized.value = await appWindow.isMaximized();
}

async function toggleMaximizeWindow() {
  if (await appWindow.isMaximized()) await appWindow.unmaximize();
  else await appWindow.maximize();
  await syncMaximizedState();
}

function hideWindow() {
  void appWindow.hide();
}

function openSettings() {
  if (settingsOpen.value) return;
  settingsOpen.value = true;
}

function closeSettings() {
  settingsOpen.value = false;
}

function toggleSettings() {
  if (settingsOpen.value) closeSettings();
  else openSettings();
}

function updateReviewWidth() {
  reviewWidth.value = Math.max(420, Math.round(document.documentElement.clientWidth * 0.46));
}

watch(activeSessionId, () => {
  clearSessionUnread(activeSessionId.value);
  openedSubagentIds.value = [];
  selectedSubagentId.value = "";
  openedImageSources.value = [];
  selectedImageSource.value = "";
  if (reviewView.value === "image") reviewView.value = "diff";
});

watch(
  [activeSessionId, settingsOpen],
  ([sessionId, showingSettings]) => void setWindowSessionView(showingSettings ? undefined : sessionId),
  { immediate: true },
);

watch(
  () => messages.value.map((message) => `${message.id}:${message.status}:${message.toolActivities?.length ?? 0}`).join("|"),
  () => void refreshCheckpoints(),
);

onMounted(async () => {
  await syncMaximizedState();
  unlisteners.push(await appWindow.onResized(() => void syncMaximizedState()));
  unlisteners.push(await appWindow.onFocusChanged(({ payload: focused }) => {
    if (focused && !settingsOpen.value) clearSessionUnread(activeSessionId.value);
  }));
  unlisteners.push(await listen("open-workbench-settings", () => {
    openSettings();
  }));
  unlisteners.push(await listen<string>("workbench-open-session", async (event) => {
    pendingWorkbenchSessionId = event.payload;
    settingsOpen.value = false;
    if (initializing.value) return;
    await refreshSessions();
    await selectConversation(event.payload);
  }));

  try {
    await refreshSessions();
    if (pendingWorkbenchSessionId) await selectConversation(pendingWorkbenchSessionId);
    else if (sessions.value[0]) await selectConversation(sessions.value[0].sessionId);
    else await createQuickConversation();
  } finally {
    initializing.value = false;
  }

  if (pendingWorkbenchSessionId && activeSessionId.value !== pendingWorkbenchSessionId) {
    await refreshSessions();
    await selectConversation(pendingWorkbenchSessionId);
  }

  unlisteners.push(await listenChatFinished((payload) => {
    if (!payload.sessionId || payload.finishReason === "cancelled") return;
    if (payload.sessionId === activeSessionId.value) void refreshCheckpoints();
    void (async () => {
      if (await isSessionBeingViewed(payload.sessionId)) {
        clearSessionUnread(payload.sessionId);
        return;
      }
      markSessionUnread(payload.sessionId);
      await showActionableWindowsNotification(
        payload.sessionId,
        tr(settingStore.language, "notification.taskCompleted"),
        sessionDisplayName(payload.sessionId),
      );
    })();
  }));
  unlisteners.push(await listenAskUser((payload) => {
    const sessionId = payload.sessionId || activeSessionId.value;
    setPendingInteraction(sessionId, {
      kind: "ask_user",
      value: { requestId: payload.requestId, questions: payload.questions },
    });
    void notifyWhenNotViewed(
      sessionId,
      tr(settingStore.language, "notification.needsInput"),
      payload.questions[0]?.question || sessionDisplayName(sessionId),
    );
  }));
  unlisteners.push(await listenPathPermission((payload) => {
    const sessionId = payload.sessionId || activeSessionId.value;
    setPendingInteraction(sessionId, { kind: "path_permission", value: payload });
    void notifyWhenNotViewed(
      sessionId,
      tr(settingStore.language, "notification.pathPermission"),
      payload.path,
    );
  }));
  unlisteners.push(await listenToolApproval((payload) => {
    const sessionId = payload.sessionId || activeSessionId.value;
    setPendingInteraction(sessionId, { kind: "tool_approval", value: payload });
    chatStore.attachToolApprovalPreview(
      sessionId,
      payload.toolName,
      payload.preview ?? null,
      activeSessionId.value,
    );
    void notifyWhenNotViewed(
      sessionId,
      tr(settingStore.language, "notification.approval"),
      payload.title || payload.toolName,
    );
  }));
  unlisteners.push(await listenInteractionResolved((payload) => {
    const matchedSessionId = Object.entries(pendingInteractions.value).find(
      ([, interaction]) => interaction.value.requestId === payload.requestId,
    )?.[0];
    if (!matchedSessionId) return;
    removePendingInteraction(matchedSessionId, payload.requestId);
    if (matchedSessionId === activeSessionId.value) void nextTick(() => inputRef.value?.focusInput());
  }));
  unlisteners.push(await appWindow.listen("workbench-opened", () => {
    void refreshSessions();
    void inputRef.value?.focusInput();
  }));

  globalThis.addEventListener("resize", updateReviewWidth);
  globalThis.addEventListener("pointermove", moveWorkspacePointerDrag);
  globalThis.addEventListener("pointerup", finishWorkspacePointerDrag);
  globalThis.addEventListener("pointercancel", cancelWorkspacePointerDrag);
  updateReviewWidth();
});

onUnmounted(() => {
  if (workspacePointerDrag.value) clearWorkspaceLongPress(workspacePointerDrag.value);
  for (const unlisten of unlisteners) unlisten();
  globalThis.removeEventListener("resize", updateReviewWidth);
  globalThis.removeEventListener("pointermove", moveWorkspacePointerDrag);
  globalThis.removeEventListener("pointerup", finishWorkspacePointerDrag);
  globalThis.removeEventListener("pointercancel", cancelWorkspacePointerDrag);
});
</script>

<style scoped>
.workbench {
  --workbench-chrome-bg: color-mix(in srgb, var(--peek-sidebar) 92%, var(--peek-bg));
  width: 100vw;
  height: 100vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: var(--workbench-chrome-bg);
  color: var(--peek-text);
  font-family: var(--font-sans);
}
.workbench-ready-leave-active { transition: opacity 180ms ease; }
.workbench-ready-leave-to { opacity: 0; }

.titlebar {
  flex: none;
  height: 42px;
  display: grid;
  grid-template-columns: 250px minmax(0, 1fr) auto auto;
  align-items: center;
  background: var(--workbench-chrome-bg);
  user-select: none;
}

button { font: inherit; }
.brand, .new-chat-button, .session-row, .review-tabs button {
  border: 0;
  color: inherit;
  cursor: pointer;
}
.brand {
  height: 100%;
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 0 14px;
  background: transparent;
  text-align: left;
}
.brand:hover { background: var(--peek-hover-bg); }
.brand svg { color: var(--peek-muted); }
.brand strong { font-size: 13px; font-weight: 650; }
.titlebar-context { min-width: 0; overflow: hidden; padding: 0 12px; color: var(--peek-muted); font-size: 12px; text-align: center; text-overflow: ellipsis; white-space: nowrap; }
.view-actions, .window-actions { display: flex; align-items: center; gap: 2px; }
.view-actions { padding-right: 6px; }
.icon-button, .small-icon-button, .window-button, .delete-session {
  display: inline-grid;
  place-items: center;
  padding: 0;
  border: 0;
  border-radius: 5px;
  background: transparent;
  color: var(--peek-muted);
  cursor: pointer;
}
.icon-button { position: relative; width: 30px; height: 30px; }
.small-icon-button { width: 27px; height: 27px; }
.window-button { width: 42px; height: 42px; border-radius: 0; }
.windows-caption-icon {
  font-family: "Segoe Fluent Icons", "Segoe MDL2 Assets", sans-serif;
  font-size: 10px;
  line-height: 1;
}
.icon-button:hover, .icon-button.active, .small-icon-button:hover, .window-button:hover { color: var(--peek-text); background: var(--peek-hover-bg); }
.icon-button.active { color: var(--peek-accent); }
.window-button.close:hover { color: white; background: #c42b1c; }
.status-dot { position: absolute; right: 6px; bottom: 6px; width: 5px; height: 5px; border-radius: 50%; background: var(--peek-accent); }

.workspace-grid {
  flex: 1;
  min-height: 0;
  display: grid;
  grid-template-columns: 250px minmax(480px, 1fr);
  grid-template-rows: minmax(0, 1fr);
  overflow: hidden;
  background: var(--workbench-chrome-bg);
}
.embedded-settings { flex: 1; min-height: 0; }
.workspace-grid.navigation-closed { grid-template-columns: minmax(480px, 1fr); }
.workspace-grid.review-open { grid-template-columns: 250px minmax(480px, 1fr) minmax(420px, 46%); }
.workspace-grid.navigation-closed.review-open { grid-template-columns: minmax(480px, 1fr) minmax(420px, 46%); }

.navigation-pane {
  min-width: 0;
  display: flex;
  flex-direction: column;
  padding: 10px 8px 8px;
  background: var(--workbench-chrome-bg);
}
.new-chat-button {
  height: 34px;
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 0 9px;
  border-radius: 6px;
  background: transparent;
  font-size: 12px;
  font-weight: 550;
}
.new-chat-button:hover { background: color-mix(in srgb, var(--peek-text) 10%, transparent); }
.session-list { flex: 1; min-height: 0; overflow-y: auto; }
.navigation-section { margin: 2px 0 8px; }
.navigation-section-header { height: 28px; display: flex; align-items: center; justify-content: space-between; border-radius: 5px; }
.navigation-section-toggle { min-width: 0; height: 28px; display: flex; align-items: center; gap: 6px; flex: 1; padding: 0 5px; border: 0; border-radius: 5px; background: transparent; color: var(--peek-text); cursor: pointer; font-size: 11px; font-weight: 650; text-align: left; }
.navigation-section-toggle:hover { background: color-mix(in srgb, var(--peek-text) 5%, transparent); }
.navigation-section-toggle > svg:first-child { flex: none; color: var(--peek-faint); transition: transform 140ms ease; }
.navigation-section-toggle > svg:first-child.expanded { transform: rotate(90deg); }
.navigation-section-toggle span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.navigation-section-toggle small { margin-left: auto; color: var(--peek-faint); font-size: 10px; font-weight: 400; }
.section-action, .workspace-actions button { display: inline-grid; place-items: center; padding: 0; border: 0; border-radius: 5px; background: transparent; color: var(--peek-muted); cursor: pointer; }
.section-action { width: 27px; height: 27px; }
.section-action:hover, .workspace-actions button:hover { color: var(--peek-text); background: var(--peek-hover-bg); }
.section-action.active { color: var(--peek-accent); background: color-mix(in srgb, var(--peek-accent) 12%, transparent); }
.navigation-section-body { padding-top: 2px; }
.workspace-group { position: relative; margin: 2px 0 5px; }
.workspace-group.drop-before::before, .workspace-group.drop-after::after { content: ""; position: absolute; z-index: 4; left: 5px; right: 5px; height: 2px; border-radius: 2px; background: var(--peek-accent); pointer-events: none; }
.workspace-group.drop-before::before { top: -3px; }
.workspace-group.drop-after::after { bottom: -3px; }
.workspace-row { position: relative; display: flex; align-items: center; min-width: 0; height: 28px; border-radius: 5px; cursor: pointer; user-select: none; transition: background-color 120ms ease, box-shadow 120ms ease; }
.workspace-row:hover { background: color-mix(in srgb, var(--peek-text) 6%, transparent); }
.workspace-group.dragging .workspace-row { cursor: grabbing; background: color-mix(in srgb, var(--peek-accent) 12%, transparent); box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--peek-accent) 24%, transparent); }
.workspace-group.dragging .workspace-group-header { cursor: grabbing; }
.workspace-collapse { flex: none; width: 25px; height: 28px; display: inline-grid; place-items: center; padding: 0; border: 0; border-radius: 5px; background: transparent; color: var(--peek-faint); cursor: inherit; }
.workspace-group-header {
  min-width: 0;
  height: 28px;
  display: flex;
  align-items: center;
  flex: 1;
  gap: 7px;
  padding: 0 4px 0 1px;
  border: 0;
  border-radius: 5px;
  background: transparent;
  color: var(--peek-text);
  cursor: inherit;
  font-size: 11px;
  text-align: left;
}
.workspace-group-header:hover { color: var(--peek-text); }
.workspace-group-header > svg { flex: none; color: var(--peek-muted); }
.workspace-group-header span { min-width: 0; flex: 1; overflow: hidden; font-weight: 600; text-overflow: ellipsis; white-space: nowrap; }
.workspace-actions { display: flex; align-items: center; gap: 1px; padding-right: 3px; }
.workspace-actions button { width: 24px; height: 24px; }
.workspace-menu { position: absolute; z-index: 20; top: 27px; right: 4px; min-width: 150px; padding: 4px; border-radius: 6px; background: var(--peek-list-bg); box-shadow: 0 8px 24px var(--peek-shadow); }
.workspace-menu button { width: 100%; height: 28px; display: flex; align-items: center; gap: 7px; padding: 0 7px; border: 0; border-radius: 4px; background: transparent; color: var(--peek-text); cursor: pointer; font-size: 11px; text-align: left; }
.workspace-menu button:hover { background: var(--peek-hover-bg); }
.workspace-menu button.danger { color: var(--peek-danger); }

.conversation-pane { position: relative; min-width: 0; min-height: 0; display: flex; flex-direction: column; overflow: hidden; border-radius: 12px 12px 0 0; background: var(--peek-list-bg); }
.workbench-messages { box-sizing: border-box; flex: 1 1 0; width: min(100%, 900px); min-height: 0; margin: 0 auto; overflow: hidden; padding-top: 18px; }
.workbench-messages :deep(.message-list) { padding: 18px 40px 28px; gap: 20px; }
.workbench-messages :deep(.assistant-bubble) { max-width: 100%; }
.workbench-messages :deep(.user-turn) { max-width: min(76%, 680px); }
.composer-wrap { z-index: 8; flex: 0 0 auto; width: min(calc(100% - 48px), 820px); min-height: 0; max-height: min(280px, calc(100% - 24px)); margin: 0 auto clamp(10px, 2.5vh, 24px); }
.composer-wrap :deep(.chat-input-shell) { width: 100%; min-height: 0; max-height: 100%; }
.composer-wrap :deep(.input-bar) { width: 100%; max-height: 100%; }
.conversation-pane.empty-conversation .workbench-messages { visibility: hidden; pointer-events: none; }
.empty-conversation-brand { position: absolute; top: calc(50% - 318px); left: 50%; width: 128px; height: 128px; transform: translateX(-50%); pointer-events: none; }
.empty-conversation-brand img { display: block; width: 100%; height: 100%; object-fit: contain; opacity: 0.9; }
.workbench[data-theme="dark"] .empty-conversation-brand img { filter: invert(1); }
.empty-conversation-prompt { position: absolute; top: calc(50% - 190px); left: 50%; width: min(calc(100% - 48px), 820px); margin: 0; color: var(--peek-muted); font-size: 15px; line-height: 1.6; text-align: center; transform: translateX(-50%); user-select: none; }
.conversation-pane.empty-conversation .composer-wrap { position: absolute; top: 50%; left: 50%; margin: 0; transform: translate(-50%, -50%); }
.conversation-pane.empty-conversation .composer-wrap :deep(.model-picker-list) {
  max-height: max(96px, calc(50vh - 96px));
}
.context-notice { position: absolute; z-index: 9; top: 10px; left: 50%; max-width: min(720px, calc(100% - 48px)); padding: 7px 10px; border-radius: 6px; background: color-mix(in srgb, var(--peek-warning) 14%, var(--peek-surface)); color: var(--peek-text); box-shadow: 0 8px 24px var(--peek-shadow); font-size: 11px; transform: translateX(-50%); }

.review-pane { min-width: 0; min-height: 0; display: flex; flex-direction: column; overflow: hidden; background: var(--workbench-chrome-bg); box-shadow: -12px 0 34px color-mix(in srgb, #000 9%, transparent); container: workspace-sidebar / inline-size; }
.review-header { flex: none; height: 38px; display: flex; align-items: center; justify-content: space-between; padding: 0 8px; }
.review-tabs { min-width: 0; display: flex; align-items: center; gap: 2px; }
.review-tabs button { height: 28px; display: inline-flex; align-items: center; gap: 6px; padding: 0 9px; border-radius: 5px; background: transparent; color: var(--peek-muted); font-size: 10px; }
.review-tabs button:hover { color: var(--peek-text); background: var(--peek-hover-bg); }
.review-tabs button.active { color: var(--peek-active-fg); background: color-mix(in srgb, var(--peek-accent) 13%, transparent); }
.review-pane > :deep(aside) { flex: 1; min-height: 0; }
.spinning { animation: spin 700ms linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }

@media (max-width: 1120px) {
  .titlebar { grid-template-columns: 210px minmax(0, 1fr) auto auto; }
  .workspace-grid { grid-template-columns: 210px minmax(480px, 1fr); }
  .workspace-grid.review-open { grid-template-columns: minmax(460px, 1fr) minmax(420px, 48%); }
  .workspace-grid.review-open .navigation-pane { display: none; }
}

@media (max-height: 700px) {
  .workbench-messages :deep(.message-list) { padding-bottom: 14px; }
  .composer-wrap { width: min(calc(100% - 28px), 820px); max-height: calc(100% - 12px); margin-bottom: 8px; }
}
</style>

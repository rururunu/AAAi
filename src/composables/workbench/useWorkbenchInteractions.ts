import { computed, ref, type Ref } from "vue";
import type { WebviewWindow } from "@tauri-apps/api/webviewWindow";

import type { AskUserSession, PathPermissionSession } from "@/components/chat/ChatInputBar.vue";
import {
  respondAskUser,
  respondPathPermission,
  respondToolApproval,
  showInteractionNotification,
} from "@/services/ipc";
import { tr } from "@/services/i18n";
import { useChatStore } from "@/stores/chat";
import { useSettingStore } from "@/stores/setting";
import type {
  ChatSessionSummary,
  PathPermissionDecision,
  ToolApprovalDecision,
  ToolApprovalSession,
} from "@/types/chat";
import type { PendingInteraction } from "./types";
import type { WorkbenchLabels } from "./useWorkbenchLabels";

export interface UseWorkbenchInteractionsOptions {
  activeSessionId: Ref<string>;
  sessions: Ref<ChatSessionSummary[]>;
  settingsOpen: Ref<boolean>;
  labels: WorkbenchLabels["labels"];
  appWindow: WebviewWindow;
}

/**
 * Unresolved ask-user / path-permission / tool-approval interactions per
 * session, unread-session tracking, and the Windows notification helpers
 * that surface both while the workbench window is not being looked at.
 */
export function useWorkbenchInteractions(options: UseWorkbenchInteractionsOptions) {
  const { activeSessionId, sessions, settingsOpen, labels, appWindow } = options;
  const chatStore = useChatStore();
  const settingStore = useSettingStore();

  const pendingInteractions = ref<Record<string, PendingInteraction>>({});
  const unreadSessionIds = ref(new Set<string>());

  const attentionSessionIds = computed(() => Object.keys(pendingInteractions.value));
  const unreadSessionIdList = computed(() => [...unreadSessionIds.value]);
  const activePendingInteraction = computed(() => pendingInteractions.value[activeSessionId.value]);
  const askUserSession = computed<AskUserSession | null>(() =>
    activePendingInteraction.value?.kind === "ask_user"
      ? activePendingInteraction.value.value
      : null,
  );
  const pathPermissionSession = computed<PathPermissionSession | null>(() =>
    activePendingInteraction.value?.kind === "path_permission"
      ? activePendingInteraction.value.value
      : null,
  );
  const toolApprovalSession = computed<ToolApprovalSession | null>(() =>
    activePendingInteraction.value?.kind === "tool_approval"
      ? activePendingInteraction.value.value
      : null,
  );

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
      sessionId !== activeSessionId.value ||
      settingsOpen.value ||
      document.visibilityState !== "visible"
    )
      return false;
    return (await appWindow.isVisible()) && (await appWindow.isFocused());
  }

  function sessionDisplayName(sessionId: string) {
    return (
      sessions.value.find((session) => session.sessionId === sessionId)?.preview ||
      labels.value.untitled
    );
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

  function isAlreadyResolvedError(error: unknown) {
    return String(error).includes("already completed") || String(error).includes("not found");
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

  return {
    pendingInteractions,
    unreadSessionIds,
    attentionSessionIds,
    unreadSessionIdList,
    activePendingInteraction,
    askUserSession,
    pathPermissionSession,
    toolApprovalSession,
    setPendingInteraction,
    removePendingInteraction,
    markSessionUnread,
    clearSessionUnread,
    isSessionBeingViewed,
    sessionDisplayName,
    showActionableWindowsNotification,
    notifyWhenNotViewed,
    isAlreadyResolvedError,
    completeAskUser,
    completePathPermission,
    completeToolApproval,
  };
}

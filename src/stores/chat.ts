import { defineStore } from "pinia";

import { chat, chatHistory } from "@/services/ipc";
import {
  normalizeChatStarted,
  normalizeMessage,
  normalizeRole,
  resolveSessionId,
  type RawChatStarted,
} from "@/services/chat/normalize";
import type {
  AskUserAnswerItem,
  ChatMessage,
  ToolActivity,
  ToolPreviewPayload,
  WorkTimelineItem,
} from "@/types/chat";

/** Mark crash-orphaned in-flight rows so the UI is not stuck "executing". */
export function settleInterruptedMessages(messages: ChatMessage[]): ChatMessage[] {
  return messages.map((message) => {
    const statusStuck =
      message.status === "pending" || message.status === "streaming";
    const toolsStuck = message.toolActivities?.some(
      (activity) => activity.status === "running",
    );
    if (!statusStuck && !toolsStuck) {
      return message;
    }
    return {
      ...message,
      status: statusStuck ? "cancelled" : message.status,
      toolActivities: message.toolActivities?.map((activity) =>
        activity.status === "running"
          ? {
              ...activity,
              status: "error" as const,
              success: false,
              result: activity.result?.trim() ? activity.result : "interrupted",
            }
          : activity,
      ),
    };
  });
}

function appendReasoningTimeline(
  timeline: WorkTimelineItem[] | undefined,
  chunk: string,
): WorkTimelineItem[] {
  const next = [...(timeline ?? [])];
  const last = next[next.length - 1];
  if (last?.type === "reasoning") {
    next[next.length - 1] = { ...last, content: last.content + chunk };
  } else {
    next.push({
      type: "reasoning",
      id: `reasoning-${Date.now()}-${next.length}`,
      content: chunk,
    });
  }
  return next;
}

function findLastMessageIndex(
  messages: ChatMessage[],
  predicate: (message: ChatMessage) => boolean,
) {
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    if (predicate(messages[index])) return index;
  }
  return -1;
}

/**
 * Thin UI store — 按 session 镜像 AI Runtime 状态。
 */
export const useChatStore = defineStore("chat", {
  state: () => ({
    sessions: {} as Record<string, ChatMessage[]>,
    sending: {} as Record<string, boolean>,
    overlayDraftSessionId: "" as string,
    contextNotices: {} as Record<string, string | undefined>,
  }),
  getters: {
    overlayMessages(state): ChatMessage[] {
      const sessionId = state.overlayDraftSessionId;
      if (!sessionId) {
        return [];
      }
      return state.sessions[sessionId] ?? [];
    },
    overlayContextNotice(state): string | undefined {
      const sessionId = state.overlayDraftSessionId;
      if (!sessionId) {
        return undefined;
      }
      return state.contextNotices[sessionId];
    },
  },
  actions: {
    setOverlayDraftSession(sessionId: string) {
      this.overlayDraftSessionId = sessionId;
    },
    setContextNotice(sessionId: string, message: string | undefined) {
      if (!sessionId) {
        return;
      }
      this.contextNotices = {
        ...this.contextNotices,
        [sessionId]: message,
      };
    },
    setSessionMessages(sessionId: string, messages: ChatMessage[]) {
      if (!sessionId) {
        return;
      }
      this.sessions = {
        ...this.sessions,
        [sessionId]: messages,
      };
    },
    resolveOverlaySessionId(preferred?: string) {
      return resolveSessionId(this.overlayDraftSessionId, preferred);
    },
    upsertMessage(message: ChatMessage) {
      const sessionId = message.sessionId;
      if (!sessionId) {
        return false;
      }

      const normalized: ChatMessage = {
        ...message,
        sessionId,
        role: normalizeRole(message.role),
      };
      const messages = this.sessions[sessionId] ?? [];
      const index = messages.findIndex((item) => item.id === normalized.id);

      if (index === -1) {
        this.setSessionMessages(sessionId, [...messages, normalized]);
        return true;
      }

      const next = [...messages];
      next[index] = normalized;
      this.setSessionMessages(sessionId, next);
      return true;
    },
    stageAskUserAnswer(sessionId: string, items: AskUserAnswerItem[]) {
      const normalized = items
        .map((item) => ({
          header: item.header?.trim() || undefined,
          selected: item.selected.map((v) => v.trim()).filter(Boolean),
          userSupplement: Boolean(item.userSupplement),
        }))
        .filter(
          (item) =>
            item.userSupplement || item.selected.length > 0,
        );
      if (normalized.length === 0) {
        return;
      }

      const resolvedSessionId = this.resolveOverlaySessionId(sessionId);
      const messages = this.sessions[resolvedSessionId];
      if (!messages) {
        return;
      }

      // 挂在当前轮的 assistant 消息上，渲染在工具卡片之后、AI 正文之前
      let targetIndex = -1;
      for (let i = messages.length - 1; i >= 0; i -= 1) {
        const message = messages[i];
        if (normalizeRole(message.role) !== "assistant") {
          continue;
        }
        if (
          message.toolActivities?.some(
            (activity) => activity.toolName === "ask_user",
          )
        ) {
          targetIndex = i;
          break;
        }
      }
      if (targetIndex === -1) {
        for (let i = messages.length - 1; i >= 0; i -= 1) {
          if (normalizeRole(messages[i].role) === "assistant") {
            targetIndex = i;
            break;
          }
        }
      }
      if (targetIndex === -1) {
        return;
      }

      const next = [...messages];
      next[targetIndex] = {
        ...next[targetIndex],
        askUserAnswer: normalized,
      };
      this.setSessionMessages(resolvedSessionId, next);
    },
    stageUserMessage(sessionId: string, content: string) {
      const trimmed = content.trim();
      if (!trimmed) {
        return;
      }

      this.upsertMessage({
        id: `local-user-${Date.now()}`,
        sessionId,
        role: "user",
        content: trimmed,
        status: "done",
        timestamp: Date.now(),
      });
    },
    stageTurn(sessionId: string, content: string) {
      const trimmed = content.trim();
      if (!trimmed) return;
      const token = `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
      const timestamp = Date.now();
      const messages = this.sessions[sessionId] ?? [];
      this.setSessionMessages(sessionId, [
        ...messages,
        {
          id: `local-user-${token}`,
          sessionId,
          role: "user",
          content: trimmed,
          status: "done",
          timestamp,
        },
        {
          id: `local-assistant-${token}`,
          sessionId,
          role: "assistant",
          content: "",
          status: "pending",
          timestamp: timestamp + 1,
        },
      ]);
    },
    mergeSession(fromSessionId: string, toSessionId: string) {
      if (!fromSessionId || !toSessionId || fromSessionId === toSessionId) {
        return;
      }

      const source = this.sessions[fromSessionId] ?? [];
      const target = this.sessions[toSessionId] ?? [];
      if (source.length === 0) {
        return;
      }

      const merged = [...target];
      for (const message of source) {
        const index = merged.findIndex((item) => item.id === message.id);
        if (index === -1) {
          merged.push({ ...message, sessionId: toSessionId });
        } else {
          merged[index] = { ...message, sessionId: toSessionId };
        }
      }

      const nextSessions = { ...this.sessions, [toSessionId]: merged };
      delete nextSessions[fromSessionId];
      this.sessions = nextSessions;

      if (this.overlayDraftSessionId === fromSessionId) {
        this.overlayDraftSessionId = toSessionId;
      }
    },
    applyChatStarted(payload: RawChatStarted) {
      const normalized = normalizeChatStarted(payload);
      if (!normalized) {
        return;
      }

      const eventSessionId = normalized.sessionId;
      const targetSessionId = this.resolveOverlaySessionId(eventSessionId);
      const userMessage = {
        ...normalized.userMessage,
        sessionId: targetSessionId,
      };
      const assistantMessage = {
        ...normalized.assistantMessage,
        sessionId: targetSessionId,
      };

      if (eventSessionId !== targetSessionId) {
        this.mergeSession(eventSessionId, targetSessionId);
      }

      let messages = [...(this.sessions[targetSessionId] ?? [])];

      const localUserIndex = findLastMessageIndex(messages,
        (item) => item.id.startsWith("local-user-") && item.content === userMessage.content,
      );
      if (localUserIndex !== -1) {
        messages[localUserIndex] = userMessage;
      } else {
        const existingUserIndex = messages.findIndex(
          (item) => item.id === userMessage.id,
        );
        if (existingUserIndex === -1) {
          messages.push(userMessage);
        } else {
          messages[existingUserIndex] = userMessage;
        }
      }

      let assistantIndex = messages.findIndex(
        (item) => item.id === assistantMessage.id,
      );
      if (assistantIndex === -1) {
        assistantIndex = findLastMessageIndex(messages,
          (item) => item.id.startsWith("local-assistant-") && item.status === "pending",
        );
      }
      if (assistantIndex === -1) {
        messages.push(assistantMessage);
      } else {
        messages[assistantIndex] = assistantMessage;
      }

      this.setSessionMessages(targetSessionId, messages);
      this.overlayDraftSessionId = targetSessionId;
      this.sending[targetSessionId] = true;
    },
    reconcileOptimisticIds(
      sessionId: string,
      userMessageId: string,
      assistantMessageId: string,
    ) {
      const messages = [...(this.sessions[sessionId] ?? [])];
      const localUserIndex = findLastMessageIndex(messages, (item) => item.id.startsWith("local-user-"));
      const localAssistantIndex = findLastMessageIndex(messages, (item) => item.id.startsWith("local-assistant-"));
      let changed = false;
      if (localUserIndex !== -1) {
        messages[localUserIndex] = { ...messages[localUserIndex], id: userMessageId };
        changed = true;
      }
      if (localAssistantIndex !== -1) {
        messages[localAssistantIndex] = { ...messages[localAssistantIndex], id: assistantMessageId };
        changed = true;
      }
      if (changed) this.setSessionMessages(sessionId, messages);
    },
    failOptimisticSend(sessionId: string, error: unknown) {
      const messages = [...(this.sessions[sessionId] ?? [])];
      const index = findLastMessageIndex(
        messages,
        (item) => normalizeRole(item.role) === "assistant" && item.status === "pending",
      );
      if (index === -1) return;
      messages[index] = {
        ...messages[index],
        content: `发送失败：${String(error)}`,
        status: "error",
      };
      this.setSessionMessages(sessionId, messages);
    },
    appendDelta(
      sessionId: string,
      messageId: string,
      delta: string,
      fallbackSessionId?: string,
    ) {
      const resolvedSessionId = this.resolveOverlaySessionId(
        resolveSessionId(sessionId, fallbackSessionId),
      );
      const messages = this.sessions[resolvedSessionId];
      if (!messages) {
        return;
      }

      const index = messages.findIndex((item) => item.id === messageId);
      if (index === -1) {
        return;
      }

      const next = [...messages];
      next[index] = {
        ...next[index],
        content: next[index].content + delta,
        status: "streaming",
      };
      this.setSessionMessages(resolvedSessionId, next);
    },
    appendReasoning(
      sessionId: string,
      messageId: string,
      chunk: string,
      fallbackSessionId?: string,
    ) {
      const resolvedSessionId = this.resolveOverlaySessionId(
        resolveSessionId(sessionId, fallbackSessionId),
      );
      const messages = this.sessions[resolvedSessionId];
      if (!messages) {
        return;
      }

      const index = messages.findIndex((item) => item.id === messageId);
      if (index === -1) {
        return;
      }

      const next = [...messages];
      const current = next[index];
      next[index] = {
        ...current,
        reasoning: (current.reasoning ?? "") + chunk,
        workTimeline: appendReasoningTimeline(current.workTimeline, chunk),
        status: current.status === "pending" ? "streaming" : current.status,
      };
      this.setSessionMessages(resolvedSessionId, next);
    },
    applyStreamDeltas(
      updates: Array<{
        sessionId: string;
        messageId: string;
        contentDelta?: string;
        reasoningDelta?: string;
        fallbackSessionId?: string;
      }>,
    ) {
      if (updates.length === 0) {
        return;
      }

      const grouped = new Map<
        string,
        Map<
          string,
          { contentDelta: string; reasoningDelta: string; fallbackSessionId?: string }
        >
      >();

      for (const update of updates) {
        const resolvedSessionId = this.resolveOverlaySessionId(
          resolveSessionId(update.sessionId, update.fallbackSessionId),
        );
        const byMessage =
          grouped.get(resolvedSessionId) ??
          new Map<
            string,
            { contentDelta: string; reasoningDelta: string; fallbackSessionId?: string }
          >();

        const current = byMessage.get(update.messageId) ?? {
          contentDelta: "",
          reasoningDelta: "",
          fallbackSessionId: update.fallbackSessionId,
        };

        if (update.contentDelta) {
          current.contentDelta += update.contentDelta;
        }
        if (update.reasoningDelta) {
          current.reasoningDelta += update.reasoningDelta;
        }

        byMessage.set(update.messageId, current);
        grouped.set(resolvedSessionId, byMessage);
      }

      for (const [resolvedSessionId, byMessage] of grouped) {
        const messages = this.sessions[resolvedSessionId];
        if (!messages) {
          continue;
        }

        const next = [...messages];
        let changed = false;

        for (const [messageId, delta] of byMessage) {
          const index = next.findIndex((item) => item.id === messageId);
          if (index === -1) {
            continue;
          }

          const current = next[index];
          next[index] = {
            ...current,
            content: current.content + delta.contentDelta,
            reasoning:
              delta.reasoningDelta.length > 0
                ? (current.reasoning ?? "") + delta.reasoningDelta
                : current.reasoning,
            workTimeline:
              delta.reasoningDelta.length > 0
                ? appendReasoningTimeline(current.workTimeline, delta.reasoningDelta)
                : current.workTimeline,
            status:
              current.status === "pending" || delta.contentDelta || delta.reasoningDelta
                ? "streaming"
                : current.status,
          };
          changed = true;
        }

        if (changed) {
          this.setSessionMessages(resolvedSessionId, next);
        }
      }
    },
    finishMessage(
      sessionId: string,
      messageId: string,
      content: string,
      fallbackSessionId?: string,
      reasoning?: string,
    ) {
      const resolvedSessionId = this.resolveOverlaySessionId(
        resolveSessionId(sessionId, fallbackSessionId),
      );
      const messages = this.sessions[resolvedSessionId];
      if (!messages) {
        this.clearSendingMany([
          sessionId,
          resolvedSessionId,
          fallbackSessionId,
          this.overlayDraftSessionId,
        ]);
        return;
      }

      const index = messages.findIndex((item) => item.id === messageId);
      if (index === -1) {
        this.clearSendingMany([
          sessionId,
          resolvedSessionId,
          fallbackSessionId,
          this.overlayDraftSessionId,
        ]);
        return;
      }

      const next = [...messages];
      next[index] = {
        ...next[index],
        content,
        status: "done",
        ...(reasoning !== undefined ? { reasoning } : {}),
      };
      this.setSessionMessages(resolvedSessionId, next);
      this.clearSendingMany([
        sessionId,
        resolvedSessionId,
        fallbackSessionId,
        this.overlayDraftSessionId,
      ]);
    },
    upsertToolActivity(
      sessionId: string,
      messageId: string,
      activity: ToolActivity,
      fallbackSessionId?: string,
    ) {
      const resolvedSessionId = this.resolveOverlaySessionId(
        resolveSessionId(sessionId, fallbackSessionId),
      );
      const messages = this.sessions[resolvedSessionId];
      if (!messages) {
        return;
      }

      const index = messages.findIndex((item) => item.id === messageId);
      if (index === -1) {
        return;
      }

      const current = messages[index];
      const activities = [...(current.toolActivities ?? [])];
      const existingIndex = activities.findIndex((item) => item.id === activity.id);
      const isNewActivity = existingIndex === -1;
      if (existingIndex === -1) {
        activities.push(activity);
      } else {
        activities[existingIndex] = { ...activities[existingIndex], ...activity };
      }

      const next = [...messages];
      next[index] = {
        ...current,
        toolActivities: activities,
        workTimeline: isNewActivity
          ? [
              ...(current.workTimeline ?? []),
              {
                type: "tool" as const,
                id: `tool-${activity.id}`,
                toolActivityId: activity.id,
              },
            ]
          : current.workTimeline,
        status:
          current.status === "pending" || current.status === "streaming"
            ? "streaming"
            : current.status,
      };
      this.setSessionMessages(resolvedSessionId, next);
    },
    attachToolApprovalPreview(
      sessionId: string,
      toolName: string,
      preview: ToolPreviewPayload | null,
      fallbackSessionId?: string,
    ) {
      const resolvedSessionId = this.resolveOverlaySessionId(
        resolveSessionId(sessionId, fallbackSessionId),
      );
      const messages = this.sessions[resolvedSessionId];
      if (!messages?.length) {
        return;
      }

      for (let messageIndex = messages.length - 1; messageIndex >= 0; messageIndex -= 1) {
        const message = messages[messageIndex];
        const activities = message.toolActivities;
        if (!activities?.length) {
          continue;
        }
        for (let activityIndex = activities.length - 1; activityIndex >= 0; activityIndex -= 1) {
          const activity = activities[activityIndex];
          if (activity.status !== "running" || activity.toolName !== toolName) {
            continue;
          }
          const nextActivities = [...activities];
          nextActivities[activityIndex] = { ...activity, preview };
          const next = [...messages];
          next[messageIndex] = { ...message, toolActivities: nextActivities };
          this.setSessionMessages(resolvedSessionId, next);
          return;
        }
      }
    },
    failMessage(
      sessionId: string,
      messageId: string,
      error: string,
      fallbackSessionId?: string,
    ) {
      const resolvedSessionId = this.resolveOverlaySessionId(
        resolveSessionId(sessionId, fallbackSessionId),
      );
      const messages = this.sessions[resolvedSessionId];
      if (!messages) {
        this.clearSendingMany([
          sessionId,
          resolvedSessionId,
          fallbackSessionId,
          this.overlayDraftSessionId,
        ]);
        return;
      }

      const index = messages.findIndex((item) => item.id === messageId);
      if (index === -1) {
        this.clearSendingMany([
          sessionId,
          resolvedSessionId,
          fallbackSessionId,
          this.overlayDraftSessionId,
        ]);
        return;
      }

      const next = [...messages];
      next[index] = {
        ...next[index],
        content: error,
        status: "error",
      };
      this.setSessionMessages(resolvedSessionId, next);
      this.clearSendingMany([
        sessionId,
        resolvedSessionId,
        fallbackSessionId,
        this.overlayDraftSessionId,
      ]);
    },
    clearSending(sessionId: string) {
      if (!sessionId || !this.sending[sessionId]) {
        return;
      }
      const next = { ...this.sending };
      delete next[sessionId];
      this.sending = next;
    },
    clearSendingMany(sessionIds: Array<string | undefined | null>) {
      const ids = sessionIds.filter((id): id is string => Boolean(id));
      if (ids.length === 0) {
        return;
      }
      const next = { ...this.sending };
      let changed = false;
      for (const id of ids) {
        if (next[id]) {
          delete next[id];
          changed = true;
        }
      }
      if (changed) {
        this.sending = next;
      }
    },
    hasActiveAssistantResponse(sessionId: string) {
      return (this.sessions[sessionId] ?? []).some(
        (message) =>
          normalizeRole(message.role) === "assistant" &&
          (message.status === "pending" || message.status === "streaming"),
      );
    },
    completeAskUserToolActivities(sessionId: string, answer?: string) {
      const resolvedSessionId = this.resolveOverlaySessionId(sessionId);
      const messages = this.sessions[resolvedSessionId];
      if (!messages) {
        return;
      }

      let changed = false;
      const next = messages.map((message) => {
        const activities = message.toolActivities;
        if (!activities?.some(
          (activity) =>
            activity.toolName === "ask_user" && activity.status === "running",
        )) {
          return message;
        }

        changed = true;
        return {
          ...message,
          toolActivities: activities.map((activity) =>
            activity.toolName === "ask_user" && activity.status === "running"
              ? {
                  ...activity,
                  status: "done" as const,
                  success: true,
                  ...(answer ? { result: answer } : {}),
                }
              : activity,
          ),
        };
      });

      if (changed) {
        this.setSessionMessages(resolvedSessionId, next);
      }
    },
    async loadHistory(sessionId: string) {
      try {
        const response = await chatHistory({ sessionId });
        const messages = response.messages
          .map((message) => normalizeMessage(message, sessionId))
          .filter((message): message is ChatMessage => message !== null);
        // If this session is not actively streaming in the current process,
        // treat leftover pending/running rows from a crash as interrupted.
        this.setSessionMessages(
          sessionId,
          this.sending[sessionId] ? messages : settleInterruptedMessages(messages),
        );
        if (!this.sending[sessionId]) {
          this.clearSending(sessionId);
        }
      } catch (error) {
        console.error("chat_history failed:", error);
        this.setSessionMessages(sessionId, []);
      }
    },
    settleInterruptedSession(sessionId: string) {
      const messages = this.sessions[sessionId];
      if (!messages) {
        this.clearSending(sessionId);
        return;
      }
      this.setSessionMessages(sessionId, settleInterruptedMessages(messages));
      this.clearSending(sessionId);
    },
    async send(
      message: string,
      sessionId: string,
      options?: { staged?: boolean },
    ) {
      const trimmed = message.trim();
      if (
        !trimmed ||
        this.sending[sessionId] ||
        (!options?.staged && this.hasActiveAssistantResponse(sessionId))
      ) {
        return false;
      }

      this.overlayDraftSessionId = sessionId;

      if (!options?.staged) {
        this.stageTurn(sessionId, trimmed);
      }

      this.sending[sessionId] = true;
      try {
        const response = await chat({
          message: trimmed,
          sessionId,
        });
        this.reconcileOptimisticIds(
          sessionId,
          response.userMessageId,
          response.assistantMessageId,
        );
        if (response.sessionId && response.sessionId !== sessionId) {
          this.mergeSession(response.sessionId, sessionId);
          this.sending[sessionId] = true;
        }
        return true;
      } catch (error) {
        console.error("chat failed:", error);
        this.failOptimisticSend(sessionId, error);
        this.clearSending(sessionId);
        return false;
      }
    },
  },
});

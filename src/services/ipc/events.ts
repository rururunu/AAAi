import type { UnlistenFn } from "@tauri-apps/api/event";
import { listen } from "@tauri-apps/api/event";

import type {
  AskUserEvent,
  ChatContextNoticeEvent,
  ChatDeltaEvent,
  ChatErrorEvent,
  ChatFinishedEvent,
  ChatReasoningEvent,
  ChatStartedEvent,
  ChatStatusEvent,
  ChatUserContentEvent,
  PathPermissionEvent,
  PlanModeChangedEvent,
  TaskListUpdatedEvent,
  ToolActivityEvent,
  ToolApprovalEvent,
} from "@/types/chat";
import { IPC_EVENTS, type IpcEvent } from "@/types/ipc";
import type { AppSettings } from "@/types/setting";

export function listenIpcEvent<TPayload>(
  event: IpcEvent,
  handler: (payload: TPayload) => void,
): Promise<UnlistenFn> {
  return listen<TPayload>(event, (event) => {
    handler(event.payload);
  });
}

export function listenSettingsChanged(handler: (settings: AppSettings) => void) {
  return listenIpcEvent<AppSettings>(IPC_EVENTS.settingsChanged, handler);
}

export function listenChatStarted(handler: (payload: ChatStartedEvent) => void) {
  return listenIpcEvent<ChatStartedEvent>(IPC_EVENTS.chatStarted, handler);
}

export function listenChatDelta(handler: (payload: ChatDeltaEvent) => void) {
  return listenIpcEvent<ChatDeltaEvent>(IPC_EVENTS.chatDelta, handler);
}

export function listenChatReasoning(handler: (payload: ChatReasoningEvent) => void) {
  return listenIpcEvent<ChatReasoningEvent>(IPC_EVENTS.chatReasoning, handler);
}

export function listenChatStatus(handler: (payload: ChatStatusEvent) => void) {
  return listenIpcEvent<ChatStatusEvent>(IPC_EVENTS.chatStatus, handler);
}

export function listenChatUserContent(handler: (payload: ChatUserContentEvent) => void) {
  return listenIpcEvent<ChatUserContentEvent>(IPC_EVENTS.chatUserContent, handler);
}

export function listenChatFinished(handler: (payload: ChatFinishedEvent) => void) {
  return listenIpcEvent<ChatFinishedEvent>(IPC_EVENTS.chatFinished, handler);
}

export function listenChatError(handler: (payload: ChatErrorEvent) => void) {
  return listenIpcEvent<ChatErrorEvent>(IPC_EVENTS.chatError, handler);
}

export function listenChatContextNotice(
  handler: (payload: ChatContextNoticeEvent) => void,
) {
  return listenIpcEvent<ChatContextNoticeEvent>(
    IPC_EVENTS.chatContextNotice,
    handler,
  );
}

export function listenOverlayShown(handler: () => void) {
  return listenIpcEvent(IPC_EVENTS.overlayShown, handler);
}

export function listenOverlayHidden(handler: () => void) {
  return listenIpcEvent(IPC_EVENTS.overlayHidden, handler);
}

export function listenSettingsOpened(handler: () => void) {
  return listenIpcEvent(IPC_EVENTS.settingsOpened, handler);
}

export function listenAskUser(handler: (payload: AskUserEvent) => void) {
  return listenIpcEvent<AskUserEvent>(IPC_EVENTS.askUser, handler);
}

export function listenPathPermission(handler: (payload: PathPermissionEvent) => void) {
  return listenIpcEvent<PathPermissionEvent>(IPC_EVENTS.pathPermission, handler);
}

export function listenToolApproval(handler: (payload: ToolApprovalEvent) => void) {
  return listenIpcEvent<ToolApprovalEvent>(IPC_EVENTS.toolApproval, handler);
}

export function listenPlanModeChanged(handler: (payload: PlanModeChangedEvent) => void) {
  return listenIpcEvent<PlanModeChangedEvent>(IPC_EVENTS.planModeChanged, handler);
}

export function listenToolStarted(handler: (payload: ToolActivityEvent) => void) {
  return listenIpcEvent<ToolActivityEvent>(IPC_EVENTS.toolStarted, handler);
}

export function listenToolFinished(handler: (payload: ToolActivityEvent) => void) {
  return listenIpcEvent<ToolActivityEvent>(IPC_EVENTS.toolFinished, handler);
}

export function listenTaskListUpdated(handler: (payload: TaskListUpdatedEvent) => void) {
  return listenIpcEvent<TaskListUpdatedEvent>(IPC_EVENTS.taskListUpdated, handler);
}

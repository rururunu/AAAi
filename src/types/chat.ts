export type Role = "system" | "user" | "assistant" | "tool";

export type MessageStatus =
  | "pending"
  | "streaming"
  | "done"
  | "error"
  | "cancelled";

export interface AskUserAnswerItem {
  header?: string;
  selected: string[];
  userSupplement?: boolean;
}

/** 与 Rust Runtime `ChatMessage` 对齐 */
export interface ChatMessage {
  id: string;
  sessionId: string;
  role: Role;
  content: string;
  reasoning?: string;
  workTimeline?: WorkTimelineItem[];
  toolActivities?: ToolActivity[];
  askUserAnswer?: AskUserAnswerItem[];
  /** Ephemeral UI status from backend (not persisted). e.g. "analyzing_images" */
  activityStatus?: string;
  status: MessageStatus;
  timestamp: number;
}

export type WorkTimelineItem =
  | { type: "reasoning"; id: string; content: string }
  | { type: "tool"; id: string; toolActivityId: string };

export interface ChatSendRequest {
  message: string;
  sessionId?: string;
}

/** 与 Rust `RequestContext` 对齐 — overlay 唤起时采集的上下文 */
export interface CapturedContext {
  selection?: string;
  selectedFiles?: string[];
  selectedImages?: string[];
  activeWindow?: string;
  workspace?: { name: string; root: string };
  clipboard?: string;
}

export interface ChatSendResponse {
  sessionId: string;
  userMessageId: string;
  assistantMessageId: string;
}

export interface ChatCancelRequest {
  messageId: string;
}

export interface ChatStartedEvent {
  sessionId: string;
  userMessage: ChatMessage;
  assistantMessage: ChatMessage;
}

export interface ChatDeltaEvent {
  sessionId: string;
  messageId: string;
  delta: string;
}

export interface ChatReasoningEvent {
  sessionId: string;
  messageId: string;
  content: string;
}

export interface ChatStatusEvent {
  sessionId: string;
  messageId: string;
  kind: string;
}

export interface ChatUserContentEvent {
  sessionId: string;
  messageId: string;
  content: string;
}

export interface ChatFinishedEvent {
  sessionId: string;
  messageId: string;
  content: string;
  reasoning?: string;
  finishReason?: string;
}

export interface ChatErrorEvent {
  sessionId: string;
  messageId: string;
  message: string;
}

export interface ChatContextNoticeEvent {
  sessionId: string;
  kind: "approaching-limit" | "compacted" | string;
  message: string;
  usageRatio: number;
  foldedMessages?: number;
}

export interface ContextUsageSnapshot {
  usageRatio: number;
  estimatedTokens: number;
  contextWindowTokens: number;
}

export interface ContextUsageRequest {
  sessionId?: string;
  draftMessage?: string;
  context?: CapturedContext;
}

export interface ContextUsageResponse {
  usageRatio: number;
  estimatedTokens: number;
  contextWindowTokens: number;
}

export interface ChatHistoryRequest {
  sessionId?: string;
}

export interface ChatHistoryResponse {
  sessionId: string;
  messages: ChatMessage[];
}

export interface ChatSessionSummary {
  sessionId: string;
  workspaceId?: string;
  preview: string;
  messageCount: number;
  updatedAt: number;
}

export interface ListChatSessionsResponse {
  sessions: ChatSessionSummary[];
}

export interface ChatModelInfo {
  id: string;
  ownedBy: string;
  /** Stable provider key for UI icons (e.g. `"deepseek"`). */
  provider: string;
}

export interface AskUserOption {
  label: string;
  description?: string;
}

export interface AskUserQuestion {
  header: string;
  question: string;
  options: AskUserOption[];
  multiSelect?: boolean;
}

/** UI-only display shape for a rendered AskUser option row (adds slug/skip for picker use). */
export type AskDisplayOption = {
  label: string;
  slug: string;
  description?: string;
  isSkip?: boolean;
};

export interface AskUserEvent {
  sessionId: string;
  requestId: string;
  questions: AskUserQuestion[];
}

export interface RespondAskUserRequest {
  requestId: string;
  answer: string;
}

export interface PathPermissionEvent {
  sessionId: string;
  requestId: string;
  path: string;
  operation: "read" | "write" | string;
  toolName: string;
}

export interface RespondPathPermissionRequest {
  requestId: string;
  decision: PathPermissionDecision;
}

export type PathPermissionDecision = "allow_once" | "allow_always" | "deny";

export type ToolApprovalDecision = "allow_once" | "allow_session" | "deny";

export interface ToolPreviewPayload {
  path: string;
  kind: string;
  oldText?: string | null;
  newText?: string | null;
  unifiedDiff: string;
}

export interface ToolApprovalEvent {
  sessionId: string;
  requestId: string;
  toolName: string;
  title: string;
  arguments?: Record<string, unknown>;
  preview?: ToolPreviewPayload | null;
}

export interface ToolApprovalSession {
  requestId: string;
  toolName: string;
  title: string;
  preview?: ToolPreviewPayload | null;
}

export interface RespondToolApprovalRequest {
  requestId: string;
  decision: ToolApprovalDecision;
}

export interface PlanModeChangedEvent {
  sessionId: string;
  active: boolean;
}

export interface CheckpointInfo {
  turn: number;
  time: number;
  prompt: string;
  files: Array<{ path: string; content?: string | null }>;
  userMessageId?: string | null;
}

export type RewindRestoreMode = "code" | "conversation" | "both";

export interface RewindSessionRequest {
  sessionId: string;
  turn: number;
  restore: RewindRestoreMode;
}

export interface RewindSessionResponse {
  restoredFiles: number;
  truncatedMessages: boolean;
}

export interface ToolActivityEvent {
  sessionId: string;
  messageId: string;
  activityId: string;
  toolName: string;
  title: string;
  kind: string;
  detail?: string;
  arguments?: Record<string, unknown>;
  result?: string;
  success?: boolean;
  status: "running" | "done" | "error" | string;
}

export interface ToolActivity {
  id: string;
  toolName: string;
  title: string;
  kind: string;
  detail?: string;
  arguments?: Record<string, unknown>;
  result?: string;
  success: boolean;
  status: "running" | "done" | "error";
  /** Pre-execution preview shown in the chat card while waiting for approval. */
  preview?: ToolPreviewPayload | null;
}

export interface TaskItem {
  content: string;
  status: string;
  activeForm?: string;
  level?: number;
}

export interface TaskListUpdatedEvent {
  sessionId: string;
  tasks: TaskItem[];
}

/** @deprecated 使用 ChatMessage */
export type Message = ChatMessage;

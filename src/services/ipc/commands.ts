import type { AppInfo } from "@/types/app";
import type {
  ChatCancelRequest,
  CapturedContext,
  ChatDeltaEvent,
  ChatErrorEvent,
  ChatFinishedEvent,
  ChatHistoryRequest,
  ChatHistoryResponse,
  ChatModelInfo,
  ChatReasoningEvent,
  ChatSendRequest,
  ChatSendResponse,
  ChatStartedEvent,
  ChatStatusEvent,
  ChatUserContentEvent,
  ContextUsageRequest,
  ContextUsageResponse,
  ListChatSessionsResponse,
  RespondAskUserRequest,
  RespondPathPermissionRequest,
  RespondToolApprovalRequest,
  CheckpointInfo,
  RewindSessionRequest,
  RewindSessionResponse,
} from "@/types/chat";
import { IPC_COMMANDS } from "@/types/ipc";
import type { AppSettings, AppSettingsPatch, GeminiAuthStatus } from "@/types/setting";
import type { ResolvedVscodeTheme, VscodeThemeSummary } from "@/services/theme/vscodeThemes";
import { invoke } from "@tauri-apps/api/core";

export function ipcInvoke<TResponse>(
  command: string,
  payload?: Record<string, unknown>,
): Promise<TResponse> {
  return payload ? invoke<TResponse>(command, payload) : invoke<TResponse>(command);
}

export function openSettings() {
  return ipcInvoke<void>(IPC_COMMANDS.openSettings);
}

export function openSessionInOverlay(sessionId: string) {
  return ipcInvoke<void>(IPC_COMMANDS.openSessionInOverlay, { sessionId, session_id: sessionId });
}

export function hideOverlay(label?: string) {
  return ipcInvoke<void>(IPC_COMMANDS.hideOverlay, label ? { label } : undefined);
}

export function minimizeOverlay(label?: string) {
  return ipcInvoke<void>(IPC_COMMANDS.minimizeOverlay, label ? { label } : undefined);
}

export function closeOverlay(label: string) {
  return ipcInvoke<void>(IPC_COMMANDS.closeOverlay, { label });
}

export function exitApp() {
  return ipcInvoke<void>(IPC_COMMANDS.exitApp);
}

export function getAppSettings() {
  return ipcInvoke<AppSettings>(IPC_COMMANDS.getAppSettings);
}

export function setAppSettings(patch: AppSettingsPatch) {
  return ipcInvoke<AppSettings>(IPC_COMMANDS.setAppSettings, { patch });
}

export function listVscodeThemes() {
  return ipcInvoke<VscodeThemeSummary[]>(IPC_COMMANDS.listVscodeThemes);
}

export function loadVscodeTheme(themeId: string) {
  return ipcInvoke<ResolvedVscodeTheme>(IPC_COMMANDS.loadVscodeTheme, { themeId });
}

export function geminiAuthStatus() {
  return ipcInvoke<GeminiAuthStatus>(IPC_COMMANDS.geminiAuthStatus);
}

export function geminiOauthLogin() {
  return ipcInvoke<GeminiAuthStatus>(IPC_COMMANDS.geminiOauthLogin);
}

export function geminiOauthLogout() {
  return ipcInvoke<GeminiAuthStatus>(IPC_COMMANDS.geminiOauthLogout);
}

export function geminiImportClientSecrets(path: string) {
  return ipcInvoke<GeminiAuthStatus>(IPC_COMMANDS.geminiImportClientSecrets, { path });
}

export function getAppInfo() {
  return ipcInvoke<AppInfo>(IPC_COMMANDS.getAppInfo);
}

export function chat(request: ChatSendRequest) {
  return ipcInvoke<ChatSendResponse>(IPC_COMMANDS.chat, { request });
}

export function chatCancel(request: ChatCancelRequest) {
  return ipcInvoke<void>(IPC_COMMANDS.chatCancel, { request });
}

export function chatHistory(request: ChatHistoryRequest = {}) {
  return ipcInvoke<ChatHistoryResponse>(IPC_COMMANDS.chatHistory, { request });
}

export function listChatSessions() {
  return ipcInvoke<ListChatSessionsResponse>(IPC_COMMANDS.listChatSessions);
}

export function listChatModels() {
  return ipcInvoke<ChatModelInfo[]>(IPC_COMMANDS.listChatModels);
}

export function getContextUsage(request: ContextUsageRequest = {}) {
  return ipcInvoke<ContextUsageResponse>(IPC_COMMANDS.getContextUsage, { request });
}

export function getEnvironmentContext() {
  return ipcInvoke<CapturedContext>(IPC_COMMANDS.getEnvironmentContext);
}

export function deleteChatSession(sessionId: string) {
  return ipcInvoke<void>("delete_chat_session", { sessionId });
}

export function clearAllChatSessions() {
  return ipcInvoke<void>("clear_all_chat_sessions");
}

export function setOverlayChatMode(label: string, enabled: boolean) {
  return ipcInvoke<void>(IPC_COMMANDS.setOverlayChatMode, { label, enabled });
}

export function setOverlayPopupOpen(label: string, open: boolean) {
  return ipcInvoke<void>(IPC_COMMANDS.setOverlayPopupOpen, { label, open });
}

export function takeOverlayContext(label: string) {
  return ipcInvoke<CapturedContext | null>(IPC_COMMANDS.takeOverlayContext, { label });
}

export function openImagePreview(pathOrBase64: string) {
  return ipcInvoke<void>("open_image_preview", {
    pathOrBase64,
    path_or_base64: pathOrBase64,
  });
}

export function getPreviewImage() {
  return ipcInvoke<string>("get_preview_image");
}

export function respondAskUser(request: RespondAskUserRequest) {
  return ipcInvoke<void>(IPC_COMMANDS.respondAskUser, { request });
}

export function respondPathPermission(request: RespondPathPermissionRequest) {
  return ipcInvoke<void>(IPC_COMMANDS.respondPathPermission, { request });
}

export function respondToolApproval(request: RespondToolApprovalRequest) {
  return ipcInvoke<void>(IPC_COMMANDS.respondToolApproval, { request });
}

export function setPlanMode(sessionId: string, active: boolean) {
  return ipcInvoke<void>(IPC_COMMANDS.setPlanMode, {
    request: { sessionId, active },
  });
}

export function getPlanMode(sessionId: string) {
  return ipcInvoke<boolean>(IPC_COMMANDS.getPlanMode, {
    request: { sessionId },
  });
}

export function listCheckpoints(sessionId: string) {
  return ipcInvoke<CheckpointInfo[]>(IPC_COMMANDS.listCheckpoints, {
    request: { sessionId },
  });
}

export function rewindSession(request: RewindSessionRequest) {
  return ipcInvoke<RewindSessionResponse>(IPC_COMMANDS.rewindSession, { request });
}

export type {
  AppInfo,
  AppSettings,
  AppSettingsPatch,
  ChatCancelRequest,
  ChatDeltaEvent,
  ChatErrorEvent,
  ChatFinishedEvent,
  ChatHistoryRequest,
  ChatHistoryResponse,
  ChatModelInfo,
  ChatReasoningEvent,
  ChatSendRequest,
  ChatSendResponse,
  ChatStartedEvent,
  ChatStatusEvent,
  ChatUserContentEvent,
};

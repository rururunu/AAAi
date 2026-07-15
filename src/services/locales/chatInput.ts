import type { AppLanguage } from "@/types/setting";

export const chatInputEn = {
  "chatInput.workspacePanelTitle": "Workspace",
  "chatInput.newWorkspace": "New Workspace",
  "chatInput.noPreviousWorkspaces": "No previous workspaces",
  "chatInput.exitWorkspace": "Exit current workspace",
} as const;

export type ChatInputI18nKey = keyof typeof chatInputEn;

export type ChatInputLocalePartial = Partial<Record<ChatInputI18nKey, string>>;

export const chatInputLocales: Record<AppLanguage, ChatInputLocalePartial> = {
  "en-US": chatInputEn,
  "zh-CN": {},
  "ja-JP": {},
  "ru-RU": {},
  "de-DE": {},
  "fr-FR": {},
  "ko-KR": {},
};

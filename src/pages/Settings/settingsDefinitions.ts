import { tr } from "@/services/i18n";
import {
  settingsFieldIds,
  getSettingFieldPath,
  getSettingFieldKeywords,
  type SettingFieldId,
  type SettingsI18nKey,
} from "@/services/locales/settings";
import type { AppLanguage, CategoryId } from "@/types/setting";

export type { CategoryId };

export type SettingType =
  | "select-color"
  | "select-language"
  | "select-reasoning-effort"
  | "select-reasoning-language"
  | "select-web-search-provider"
  | "select-tool-approval-mode"
  | "select-agent-work-display"
  | "select-model"
  | "select-zoom"
  | "secret"
  | "memory-secret"
  | "search-secret"
  | "memory-text"
  | "toggle"
  | "readonly"
  | "slider"
  | "hotkey-record"
  | "collaboration-models";

export interface SettingDefinition {
  id: string;
  category: CategoryId;
  group: string;
  path: string;
  title: string;
  description: string;
  type: SettingType;
  keywords: string[];
  value?: string;
  min?: number;
  max?: number;
  step?: number;
}

export interface SettingsAppInfo {
  appName: string;
  appVersion: string;
  appIdentifier: string;
}

interface FieldCopy {
  title: string;
  description: string;
  path: string;
  keywords: string[];
}

function buildFieldCopy(language: AppLanguage): Record<SettingFieldId, FieldCopy> {
  return Object.fromEntries(
    settingsFieldIds.map((id) => {
      const title = tr(language, `settings.fields.${id}.title` as SettingsI18nKey);
      const description = tr(language, `settings.fields.${id}.description` as SettingsI18nKey);
      const entry: FieldCopy = {
        title,
        description,
        path: getSettingFieldPath(language, id, title),
        keywords: getSettingFieldKeywords(language, id),
      };
      return [id, entry];
    }),
  ) as Record<SettingFieldId, FieldCopy>;
}

export function buildSettingDefinitions(
  language: AppLanguage,
  info: SettingsAppInfo,
): SettingDefinition[] {
  const m = buildFieldCopy(language);
  const groups = {
    appearance: tr(language, "settings.groups.appearance"),
    ai: tr(language, "settings.groups.ai"),
    memory: tr(language, "settings.groups.memory"),
    search: tr(language, "settings.groups.search"),
    agent: tr(language, "settings.groups.agent"),
    plugins: tr(language, "settings.groups.plugins"),
    about: tr(language, "settings.groups.about"),
  };

  return [
    {
      id: "colorScheme",
      category: "appearance",
      group: groups.appearance,
      path: m.colorScheme.path,
      title: m.colorScheme.title,
      description: m.colorScheme.description,
      type: "select-color",
      keywords: [...m.colorScheme.keywords],
    },
    {
      id: "language",
      category: "appearance",
      group: groups.appearance,
      path: m.language.path,
      title: m.language.title,
      description: m.language.description,
      type: "select-language",
      keywords: [...m.language.keywords],
    },
    {
      id: "zoom",
      category: "appearance",
      group: groups.appearance,
      path: m.zoom.path,
      title: m.zoom.title,
      description: m.zoom.description,
      type: "select-zoom",
      keywords: [...m.zoom.keywords],
    },
    {
      id: "hardwareAccelerationEnabled",
      category: "appearance",
      group: groups.appearance,
      path: m.hardwareAccelerationEnabled.path,
      title: m.hardwareAccelerationEnabled.title,
      description: m.hardwareAccelerationEnabled.description,
      type: "toggle",
      keywords: [...m.hardwareAccelerationEnabled.keywords],
    },
    {
      id: "opacity",
      category: "appearance",
      group: groups.appearance,
      path: m.opacity.path,
      title: m.opacity.title,
      description: m.opacity.description,
      type: "slider",
      min: 10,
      max: 100,
      step: 5,
      keywords: [...m.opacity.keywords],
    },
    {
      id: "primaryHotkey",
      category: "appearance",
      group: groups.appearance,
      path: m.primaryHotkey.path,
      title: m.primaryHotkey.title,
      description: m.primaryHotkey.description,
      type: "hotkey-record",
      keywords: [...m.primaryHotkey.keywords],
    },
    {
      id: "secondaryHotkey",
      category: "appearance",
      group: groups.appearance,
      path: m.secondaryHotkey.path,
      title: m.secondaryHotkey.title,
      description: m.secondaryHotkey.description,
      type: "hotkey-record",
      keywords: [...m.secondaryHotkey.keywords],
    },
    {
      id: "defaultModel",
      category: "ai",
      group: groups.ai,
      path: m.defaultModel.path,
      title: m.defaultModel.title,
      description: m.defaultModel.description,
      type: "select-model",
      keywords: [...m.defaultModel.keywords],
    },
    {
      id: "multimodalModel",
      category: "ai",
      group: groups.ai,
      path: m.multimodalModel.path,
      title: m.multimodalModel.title,
      description: m.multimodalModel.description,
      type: "select-model",
      keywords: [...m.multimodalModel.keywords],
    },
    {
      id: "multimodalSplitAnalysis",
      category: "ai",
      group: groups.ai,
      path: m.multimodalSplitAnalysis.path,
      title: m.multimodalSplitAnalysis.title,
      description: m.multimodalSplitAnalysis.description,
      type: "toggle",
      keywords: [...m.multimodalSplitAnalysis.keywords],
    },
    {
      id: "largeContextEnabled",
      category: "ai",
      group: groups.ai,
      path: m.largeContextEnabled.path,
      title: m.largeContextEnabled.title,
      description: m.largeContextEnabled.description,
      type: "toggle",
      keywords: [...m.largeContextEnabled.keywords],
    },
    {
      id: "reasoningEffort",
      category: "ai",
      group: groups.ai,
      path: m.reasoningEffort.path,
      title: m.reasoningEffort.title,
      description: m.reasoningEffort.description,
      type: "select-reasoning-effort",
      keywords: [...m.reasoningEffort.keywords],
    },
    {
      id: "reasoningLanguage",
      category: "ai",
      group: groups.ai,
      path: m.reasoningLanguage.path,
      title: m.reasoningLanguage.title,
      description: m.reasoningLanguage.description,
      type: "select-reasoning-language",
      keywords: [...m.reasoningLanguage.keywords],
    },
    {
      id: "showReasoning",
      category: "ai",
      group: groups.ai,
      path: m.showReasoning.path,
      title: m.showReasoning.title,
      description: m.showReasoning.description,
      type: "toggle",
      keywords: [...m.showReasoning.keywords],
    },
    {
      id: "passToolReasoning",
      category: "ai",
      group: groups.ai,
      path: m.passToolReasoning.path,
      title: m.passToolReasoning.title,
      description: m.passToolReasoning.description,
      type: "toggle",
      keywords: [...m.passToolReasoning.keywords],
    },
    {
      id: "continueThinkingAfterTools",
      category: "ai",
      group: groups.ai,
      path: m.continueThinkingAfterTools.path,
      title: m.continueThinkingAfterTools.title,
      description: m.continueThinkingAfterTools.description,
      type: "toggle",
      keywords: [...m.continueThinkingAfterTools.keywords],
    },
    {
      id: "memoryEnabled",
      category: "memory",
      group: groups.memory,
      path: m.memoryEnabled.path,
      title: m.memoryEnabled.title,
      description: m.memoryEnabled.description,
      type: "toggle",
      keywords: [...m.memoryEnabled.keywords],
    },
    {
      id: "mem0ApiKey",
      category: "memory",
      group: groups.memory,
      path: m.mem0ApiKey.path,
      title: m.mem0ApiKey.title,
      description: m.mem0ApiKey.description,
      type: "memory-secret",
      keywords: [...m.mem0ApiKey.keywords],
    },
    {
      id: "mem0UserId",
      category: "memory",
      group: groups.memory,
      path: m.mem0UserId.path,
      title: m.mem0UserId.title,
      description: m.mem0UserId.description,
      type: "memory-text",
      keywords: [...m.mem0UserId.keywords],
    },
    {
      id: "mem0BaseUrl",
      category: "memory",
      group: groups.memory,
      path: m.mem0BaseUrl.path,
      title: m.mem0BaseUrl.title,
      description: m.mem0BaseUrl.description,
      type: "memory-text",
      keywords: [...m.mem0BaseUrl.keywords],
    },
    {
      id: "webSearchEnabled",
      category: "search",
      group: groups.search,
      path: m.webSearchEnabled.path,
      title: m.webSearchEnabled.title,
      description: m.webSearchEnabled.description,
      type: "toggle",
      keywords: [...m.webSearchEnabled.keywords],
    },
    {
      id: "webSearchProvider",
      category: "search",
      group: groups.search,
      path: m.webSearchProvider.path,
      title: m.webSearchProvider.title,
      description: m.webSearchProvider.description,
      type: "select-web-search-provider",
      keywords: [...m.webSearchProvider.keywords],
    },
    {
      id: "serperApiKey",
      category: "search",
      group: groups.search,
      path: m.serperApiKey.path,
      title: m.serperApiKey.title,
      description: m.serperApiKey.description,
      type: "search-secret",
      keywords: [...m.serperApiKey.keywords],
    },
    {
      id: "tavilyApiKey",
      category: "search",
      group: groups.search,
      path: m.tavilyApiKey.path,
      title: m.tavilyApiKey.title,
      description: m.tavilyApiKey.description,
      type: "search-secret",
      keywords: [...m.tavilyApiKey.keywords],
    },
    {
      id: "toolApprovalMode",
      category: "agent",
      group: groups.agent,
      path: m.toolApprovalMode.path,
      title: m.toolApprovalMode.title,
      description: m.toolApprovalMode.description,
      type: "select-tool-approval-mode",
      keywords: [...m.toolApprovalMode.keywords],
    },
    {
      id: "agentWorkDisplay",
      category: "agent",
      group: groups.agent,
      path: m.agentWorkDisplay.path,
      title: m.agentWorkDisplay.title,
      description: m.agentWorkDisplay.description,
      type: "select-agent-work-display",
      keywords: [...m.agentWorkDisplay.keywords],
    },
    {
      id: "lspEnabled",
      category: "agent",
      group: groups.agent,
      path: m.lspEnabled.path,
      title: m.lspEnabled.title,
      description: m.lspEnabled.description,
      type: "toggle",
      keywords: [...m.lspEnabled.keywords],
    },
    {
      id: "multiModelCollaboration",
      category: "agent",
      group: groups.agent,
      path: m.multiModelCollaboration.path,
      title: m.multiModelCollaboration.title,
      description: m.multiModelCollaboration.description,
      type: "collaboration-models",
      keywords: [...m.multiModelCollaboration.keywords],
    },
    {
      id: "minimalCoding",
      category: "agent",
      group: groups.agent,
      path: m.minimalCoding.path,
      title: m.minimalCoding.title,
      description: m.minimalCoding.description,
      type: "toggle",
      keywords: [...m.minimalCoding.keywords],
    },
    {
      id: "pixpinPinAiEnabled",
      category: "plugins",
      group: groups.plugins,
      path: m.pixpinPinAiEnabled.path,
      title: m.pixpinPinAiEnabled.title,
      description: m.pixpinPinAiEnabled.description,
      type: "toggle",
      keywords: [...m.pixpinPinAiEnabled.keywords],
    },
    {
      id: "snipastePinAiEnabled",
      category: "plugins",
      group: groups.plugins,
      path: m.snipastePinAiEnabled.path,
      title: m.snipastePinAiEnabled.title,
      description: m.snipastePinAiEnabled.description,
      type: "toggle",
      keywords: [...m.snipastePinAiEnabled.keywords],
    },
    {
      id: "appName",
      category: "about",
      group: groups.about,
      path: m.appName.path,
      title: m.appName.title,
      description: m.appName.description,
      type: "readonly",
      keywords: [...m.appName.keywords],
      value: info.appName,
    },
    {
      id: "appVersion",
      category: "about",
      group: groups.about,
      path: m.appVersion.path,
      title: m.appVersion.title,
      description: m.appVersion.description,
      type: "readonly",
      keywords: [...m.appVersion.keywords],
      value: info.appVersion,
    },
    {
      id: "appIdentifier",
      category: "about",
      group: groups.about,
      path: m.appIdentifier.path,
      title: m.appIdentifier.title,
      description: m.appIdentifier.description,
      type: "readonly",
      keywords: [...m.appIdentifier.keywords],
      value: info.appIdentifier,
    },
  ];
}

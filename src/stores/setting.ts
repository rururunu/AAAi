import { defineStore } from "pinia";

import { DEFAULT_CHAT_MODEL } from "@/constants/chat";
import { getAppSettings, setAppSettings } from "@/services/ipc";
import { applyOpacity } from "@/services/overlay/appearance";
import { applyVscodeTheme, clearVscodeThemeOverrides, invalidatePendingThemeLoad } from "@/services/theme/vscodeThemes";
import {
    isLightColorScheme,
    normalizeColorScheme,
    type AppLanguage,
    type AppSettings,
    type AppSettingsPatch,
    type ColorScheme,
    defaultGeminiOAuthSettings,
} from "@/types/setting";

const LEGACY_STORAGE_KEY = "peek.settings";

const defaultSettings: AppSettings = {
    colorScheme: "dark",
    vscodeTheme: "",
    language: "zh-CN",
    deepseekApiKey: "",
    geminiOauth: defaultGeminiOAuthSettings(),
    memoryEnabled: true,
    mem0ApiKey: "",
    mem0UserId: "peek-user",
    mem0BaseUrl: "https://api.mem0.ai/v1",
    webSearchEnabled: false,
    webSearchProvider: "serper",
    serperApiKey: "",
    tavilyApiKey: "",
    toolApprovalMode: "ask",
    chatMode: "agent",
    lspEnabled: false,
    lspServers: [],
    mcpServers: [],
    opacity: 100,
    chatModel: DEFAULT_CHAT_MODEL,
    multimodalModel: "gpt-4o",
    multimodalSplitAnalysis: true,
    largeContextEnabled: true,
    reasoningEffort: "high",
    reasoningLanguage: "auto",
    passToolReasoning: true,
    showReasoning: true,
    multiModelCollaboration: false,
    collaborationModels: [],
    zoom: 100,
    primaryHotkey: "Alt",
    secondaryHotkey: "Ctrl+Alt+Space",
    customProviders: [],
    pixpinPinAiEnabled: true,
    snipastePinAiEnabled: true,
};

export function applyTheme(settings: Pick<AppSettings, "colorScheme" | "vscodeTheme" | "language">) {
    const colorScheme = normalizeColorScheme(settings.colorScheme);
    invalidatePendingThemeLoad();
    clearVscodeThemeOverrides();
    document.documentElement.dataset.theme = colorScheme;
    document.documentElement.lang = settings.language;
    document.documentElement.classList.toggle("dark", !isLightColorScheme(colorScheme));
    document.documentElement.style.colorScheme = isLightColorScheme(colorScheme) ? "light" : "dark";
    if (settings.vscodeTheme?.trim()) void applyVscodeTheme(settings.vscodeTheme.trim());
}

export function applyZoom(zoom: number) {
    document.documentElement.style.zoom = String(zoom / 100);
}

export const useSettingStore = defineStore("setting", {
    state: (): AppSettings => ({ ...defaultSettings }),
    actions: {
        applyPublicSettings(settings: AppSettings) {
            this.colorScheme = normalizeColorScheme(settings.colorScheme);
            this.vscodeTheme = settings.vscodeTheme ?? "";
            this.language = settings.language;
            
            let opacityVal = settings.opacity;
            if (opacityVal === undefined && (settings as any).frostedGlass !== undefined) {
                opacityVal = (settings as any).frostedGlass ? 80 : 100;
            }
            this.opacity = opacityVal ?? 100;

            this.chatModel = settings.chatModel ?? DEFAULT_CHAT_MODEL;
            this.multimodalModel = settings.multimodalModel ?? "gpt-4o";
            this.multimodalSplitAnalysis = settings.multimodalSplitAnalysis ?? true;
            this.largeContextEnabled = settings.largeContextEnabled ?? true;
            this.reasoningEffort = settings.reasoningEffort ?? "high";
            this.reasoningLanguage = settings.reasoningLanguage ?? "auto";
            this.passToolReasoning = settings.passToolReasoning ?? true;
            this.showReasoning = settings.showReasoning ?? true;
            this.multiModelCollaboration = settings.multiModelCollaboration ?? false;
            this.collaborationModels = settings.collaborationModels ?? [];
            this.memoryEnabled = settings.memoryEnabled ?? true;
            this.mem0UserId = settings.mem0UserId ?? "peek-user";
            this.mem0BaseUrl = settings.mem0BaseUrl ?? "https://api.mem0.ai/v1";
            this.webSearchEnabled = settings.webSearchEnabled ?? false;
            this.webSearchProvider = settings.webSearchProvider ?? "serper";
            this.toolApprovalMode = settings.toolApprovalMode ?? "ask";
            this.chatMode = settings.chatMode ?? "agent";
            this.lspEnabled = settings.lspEnabled ?? false;
            this.lspServers = settings.lspServers ?? [];
            this.mcpServers = settings.mcpServers ?? [];

            let zoomVal = settings.zoom;
            if (zoomVal !== undefined && zoomVal <= 2.0) {
                zoomVal = Math.round(zoomVal * 100);
            }
            this.zoom = zoomVal ?? 100;
            this.primaryHotkey = settings.primaryHotkey ?? "Alt";
            this.secondaryHotkey = settings.secondaryHotkey ?? "Ctrl+Alt+Space";
            this.customProviders = settings.customProviders ?? [];
            this.pixpinPinAiEnabled = settings.pixpinPinAiEnabled ?? true;
            this.snipastePinAiEnabled = settings.snipastePinAiEnabled ?? true;

            applyTheme(settings);
            applyZoom(this.zoom);
            void applyOpacity(this.opacity);
        },
        applySettings(settings: AppSettings) {
            this.colorScheme = normalizeColorScheme(settings.colorScheme);
            this.vscodeTheme = settings.vscodeTheme ?? "";
            this.language = settings.language;
            this.deepseekApiKey = settings.deepseekApiKey ?? "";
            this.geminiOauth = settings.geminiOauth ?? defaultGeminiOAuthSettings();
            this.memoryEnabled = settings.memoryEnabled ?? true;
            this.mem0ApiKey = settings.mem0ApiKey ?? "";
            this.mem0UserId = settings.mem0UserId ?? "peek-user";
            this.mem0BaseUrl = settings.mem0BaseUrl ?? "https://api.mem0.ai/v1";
            this.webSearchEnabled = settings.webSearchEnabled ?? false;
            this.webSearchProvider = settings.webSearchProvider ?? "serper";
            this.serperApiKey = settings.serperApiKey ?? "";
            this.tavilyApiKey = settings.tavilyApiKey ?? "";
            this.toolApprovalMode = settings.toolApprovalMode ?? "ask";
            this.chatMode = settings.chatMode ?? "agent";
            this.lspEnabled = settings.lspEnabled ?? false;
            this.lspServers = settings.lspServers ?? [];
            this.mcpServers = settings.mcpServers ?? [];
            
            let opacityVal = settings.opacity;
            if (opacityVal === undefined && (settings as any).frostedGlass !== undefined) {
                opacityVal = (settings as any).frostedGlass ? 80 : 100;
            }
            this.opacity = opacityVal ?? 100;

            this.chatModel = settings.chatModel ?? DEFAULT_CHAT_MODEL;
            this.multimodalModel = settings.multimodalModel ?? "gpt-4o";
            this.multimodalSplitAnalysis = settings.multimodalSplitAnalysis ?? true;
            this.largeContextEnabled = settings.largeContextEnabled ?? true;
            this.reasoningEffort = settings.reasoningEffort ?? "high";
            this.reasoningLanguage = settings.reasoningLanguage ?? "auto";
            this.passToolReasoning = settings.passToolReasoning ?? true;
            this.showReasoning = settings.showReasoning ?? true;
            this.multiModelCollaboration = settings.multiModelCollaboration ?? false;
            this.collaborationModels = settings.collaborationModels ?? [];

            let zoomVal = settings.zoom;
            if (zoomVal !== undefined && zoomVal <= 2.0) {
                zoomVal = Math.round(zoomVal * 100);
            }
            this.zoom = zoomVal ?? 100;
            this.primaryHotkey = settings.primaryHotkey ?? "Alt";
            this.secondaryHotkey = settings.secondaryHotkey ?? "Ctrl+Alt+Space";
            this.customProviders = settings.customProviders ?? [];
            this.pixpinPinAiEnabled = settings.pixpinPinAiEnabled ?? true;
            this.snipastePinAiEnabled = settings.snipastePinAiEnabled ?? true;

            applyTheme(settings);
            applyZoom(this.zoom);
            void applyOpacity(this.opacity);
        },
        async load() {
            try {
                const settings = await getAppSettings();
                this.applySettings(settings);
            } catch (error) {
                console.error("get_app_settings failed:", error);
                this.applySettings(defaultSettings);
            }

            const legacy = localStorage.getItem(LEGACY_STORAGE_KEY);
            if (!legacy) {
                return;
            }

            try {
                const parsed = JSON.parse(legacy) as AppSettingsPatch;
                const settings = await setAppSettings(parsed);
                this.applySettings(settings);
            } catch (error) {
                console.error("legacy settings migration failed:", error);
            } finally {
                localStorage.removeItem(LEGACY_STORAGE_KEY);
            }
        },
        async update(partial: AppSettingsPatch) {
            const settings = await setAppSettings(partial);
            this.applySettings(settings);
        },
    },
});

export type { AppLanguage, AppSettings, AppSettingsPatch, ColorScheme };

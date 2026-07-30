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

function normalizeOpacityValue(settings: AppSettings): number {
    let opacityVal = settings.opacity;
    if (opacityVal === undefined && (settings as any).frostedGlass !== undefined) {
        opacityVal = (settings as any).frostedGlass ? 80 : 100;
    }
    return opacityVal ?? 100;
}

function normalizeZoomValue(settings: AppSettings): number {
    let zoomVal = settings.zoom;
    if (zoomVal !== undefined && zoomVal <= 2.0) {
        zoomVal = Math.round(zoomVal * 100);
    }
    return zoomVal ?? 100;
}

function applyCommonSettings(target: AppSettings, settings: AppSettings) {
    target.colorScheme = normalizeColorScheme(settings.colorScheme);
    target.vscodeTheme = settings.vscodeTheme ?? "";
    target.language = settings.language;
    target.opacity = normalizeOpacityValue(settings);
    target.chatModel = settings.chatModel ?? DEFAULT_CHAT_MODEL;
    target.multimodalModel = settings.multimodalModel ?? "gpt-4o";
    target.multimodalSplitAnalysis = settings.multimodalSplitAnalysis ?? true;
    target.largeContextEnabled = settings.largeContextEnabled ?? true;
    target.reasoningEffort = settings.reasoningEffort ?? "high";
    target.reasoningLanguage = settings.reasoningLanguage ?? "auto";
    target.passToolReasoning = settings.passToolReasoning ?? true;
    target.showReasoning = settings.showReasoning ?? true;
    target.multiModelCollaboration = settings.multiModelCollaboration ?? false;
    target.collaborationModels = settings.collaborationModels ?? [];
    target.memoryEnabled = settings.memoryEnabled ?? true;
    target.mem0UserId = settings.mem0UserId ?? "peek-user";
    target.mem0BaseUrl = settings.mem0BaseUrl ?? "https://api.mem0.ai/v1";
    target.webSearchEnabled = settings.webSearchEnabled ?? false;
    target.webSearchProvider = settings.webSearchProvider ?? "serper";
    target.toolApprovalMode = settings.toolApprovalMode ?? "ask";
    target.chatMode = settings.chatMode ?? "agent";
    target.lspEnabled = settings.lspEnabled ?? false;
    target.lspServers = settings.lspServers ?? [];
    target.mcpServers = settings.mcpServers ?? [];
    target.zoom = normalizeZoomValue(settings);
    target.primaryHotkey = settings.primaryHotkey ?? "Alt";
    target.secondaryHotkey = settings.secondaryHotkey ?? "Ctrl+Alt+Space";
    target.customProviders = settings.customProviders ?? [];
    target.pixpinPinAiEnabled = settings.pixpinPinAiEnabled ?? true;
    target.snipastePinAiEnabled = settings.snipastePinAiEnabled ?? true;
}

function applySecretSettings(target: AppSettings, settings: AppSettings) {
    target.deepseekApiKey = settings.deepseekApiKey ?? "";
    target.geminiOauth = settings.geminiOauth ?? defaultGeminiOAuthSettings();
    target.mem0ApiKey = settings.mem0ApiKey ?? "";
    target.serperApiKey = settings.serperApiKey ?? "";
    target.tavilyApiKey = settings.tavilyApiKey ?? "";
}

export const useSettingStore = defineStore("setting", {
    state: (): AppSettings => ({ ...defaultSettings }),
    actions: {
        applyPublicSettings(settings: AppSettings) {
            applyCommonSettings(this, settings);
            applyTheme(settings);
            applyZoom(this.zoom);
            void applyOpacity(this.opacity);
        },
        applySettings(settings: AppSettings) {
            applyCommonSettings(this, settings);
            applySecretSettings(this, settings);
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

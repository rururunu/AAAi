import { defineStore } from "pinia";

import { DEFAULT_CHAT_MODEL } from "@/constants/chat";
import { getAppSettings, setAppSettings } from "@/services/ipc";
import { applyOpacity } from "@/services/overlay/appearance";
import {
    DEFAULT_ACCENT_COLOR,
    isLightColorScheme,
    normalizeAccentColor,
    type AppLanguage,
    type AppSettings,
    type AppSettingsPatch,
    type ColorScheme,
    defaultGeminiOAuthSettings,
} from "@/types/setting";

const LEGACY_STORAGE_KEY = "peek.settings";

const defaultSettings: AppSettings = {
    colorScheme: "blue-black",
    customAccentColor: DEFAULT_ACCENT_COLOR,
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
    zoom: 100,
    secondaryHotkey: "Ctrl+Alt+Space",
    customProviders: [],
};

export function applyTheme(settings: Pick<AppSettings, "colorScheme" | "customAccentColor" | "language">) {
    document.documentElement.dataset.theme = settings.colorScheme;
    document.documentElement.lang = settings.language;
    document.documentElement.classList.toggle("dark", !isLightColorScheme(settings.colorScheme));
    const accent = normalizeAccentColor(settings.customAccentColor);
    const style = document.documentElement.style;
    style.setProperty("--peek-accent", accent);
    style.setProperty("--peek-send-active-bg", accent);
    style.setProperty("--peek-list-active", `color-mix(in srgb, ${accent} 15%, transparent)`);
}

export function applyZoom(zoom: number) {
    document.documentElement.style.zoom = String(zoom / 100);
}

export const useSettingStore = defineStore("setting", {
    state: (): AppSettings => ({ ...defaultSettings }),
    actions: {
        applyPublicSettings(settings: AppSettings) {
            this.colorScheme = settings.colorScheme;
            this.customAccentColor = normalizeAccentColor(settings.customAccentColor);
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
            this.secondaryHotkey = settings.secondaryHotkey ?? "Ctrl+Alt+Space";
            this.customProviders = settings.customProviders ?? [];

            applyTheme(settings);
            applyZoom(this.zoom);
            void applyOpacity(this.opacity);
        },
        applySettings(settings: AppSettings) {
            this.colorScheme = settings.colorScheme;
            this.customAccentColor = normalizeAccentColor(settings.customAccentColor);
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

            let zoomVal = settings.zoom;
            if (zoomVal !== undefined && zoomVal <= 2.0) {
                zoomVal = Math.round(zoomVal * 100);
            }
            this.zoom = zoomVal ?? 100;
            this.secondaryHotkey = settings.secondaryHotkey ?? "Ctrl+Alt+Space";
            this.customProviders = settings.customProviders ?? [];

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

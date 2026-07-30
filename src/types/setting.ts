export type ColorScheme = "dark" | "light";

export const LIGHT_COLOR_SCHEMES = new Set<ColorScheme>(["light"]);

export function normalizeColorScheme(value: unknown): ColorScheme {
    if (value === "paper" || value === "light" || value === "cream" || value === "frost") {
        return "light";
    }
    return "dark";
}

export function isLightColorScheme(scheme: ColorScheme): boolean {
    return LIGHT_COLOR_SCHEMES.has(scheme);
}

export type AppLanguage = "zh-CN" | "en-US" | "ja-JP" | "ru-RU" | "de-DE" | "fr-FR" | "ko-KR";

export type ReasoningEffort = "disabled" | "high" | "max";

export type ReasoningLanguage = "auto" | "zh" | "en";

export type WebSearchProvider = "serper" | "tavily";

export type ToolApprovalMode = "ask" | "auto" | "alwaysAllow";

/** Chat interaction mode: Agent can mutate; Ask exposes read-only tools only. */
export type ChatMode = "agent" | "ask";

export interface LspServerConfig {
    id: string;
    languages: string[];
    command: string;
    args?: string[];
    enabled?: boolean;
}

export interface McpServerConfig {
    id: string;
    /** Display name from catalog or manual entry; falls back to id in UI. */
    title?: string;
    description?: string;
    command: string;
    args?: string[];
    env?: Array<[string, string]>;
    enabled?: boolean;
}

export interface CustomProviderConfig {
    id: string;
    name: string;
    baseUrl: string;
    apiKey: string;
    /** Newline or comma-separated model IDs. */
    models: string;
}

export interface GeminiOAuthSettings {
    clientId: string;
    clientSecret: string;
    accessToken: string;
    refreshToken: string;
    expiresAt: number;
    email: string;
    projectId: string;
}

export interface GeminiAuthStatus {
    loggedIn: boolean;
    email: string;
    hasClientSecret: boolean;
    clientId: string;
}

export const DEFAULT_GEMINI_OAUTH_CLIENT_ID = "";

export const DEFAULT_GEMINI_OAUTH_CLIENT_SECRET = "";

export function defaultGeminiOAuthSettings(): GeminiOAuthSettings {
    return {
        clientId: DEFAULT_GEMINI_OAUTH_CLIENT_ID,
        clientSecret: DEFAULT_GEMINI_OAUTH_CLIENT_SECRET,
        accessToken: "",
        refreshToken: "",
        expiresAt: 0,
        email: "",
        projectId: "",
    };
}

export interface AppSettings {
    colorScheme: ColorScheme;
    vscodeTheme: string;
    language: AppLanguage;
    deepseekApiKey: string;
    geminiOauth: GeminiOAuthSettings;
    memoryEnabled: boolean;
    mem0ApiKey: string;
    mem0UserId: string;
    mem0BaseUrl: string;
    webSearchEnabled: boolean;
    webSearchProvider: WebSearchProvider;
    serperApiKey: string;
    tavilyApiKey: string;
    toolApprovalMode: ToolApprovalMode;
    chatMode: ChatMode;
    lspEnabled: boolean;
    lspServers: LspServerConfig[];
    mcpServers: McpServerConfig[];
    opacity: number;
    chatModel: string;
    chatModelProvider: string;
    multimodalModel: string;
    multimodalModelProvider: string;
    multimodalSplitAnalysis: boolean;
    /** Use 1M-token context window for compaction / turn budgets. */
    largeContextEnabled: boolean;
    reasoningEffort: ReasoningEffort;
    reasoningLanguage: ReasoningLanguage;
    /** Pass reasoning_content back on tool-call turns (DeepSeek thinking + tools). */
    passToolReasoning: boolean;
    /** Display reasoning content supplied by the model in chat. */
    showReasoning: boolean;
    multiModelCollaboration: boolean;
    collaborationModels: string[];
    zoom: number;
    primaryHotkey: string;
    secondaryHotkey: string;
    customProviders: CustomProviderConfig[];
    /** Show AI button on PixPin pin windows. */
    pixpinPinAiEnabled: boolean;
    /** Show AI button on Snipaste pin windows. */
    snipastePinAiEnabled: boolean;
}

export interface AppSettingsPatch {
    colorScheme?: ColorScheme;
    vscodeTheme?: string;
    language?: AppLanguage;
    deepseekApiKey?: string;
    geminiOauth?: GeminiOAuthSettings;
    memoryEnabled?: boolean;
    mem0ApiKey?: string;
    mem0UserId?: string;
    mem0BaseUrl?: string;
    webSearchEnabled?: boolean;
    webSearchProvider?: WebSearchProvider;
    serperApiKey?: string;
    tavilyApiKey?: string;
    toolApprovalMode?: ToolApprovalMode;
    chatMode?: ChatMode;
    lspEnabled?: boolean;
    lspServers?: LspServerConfig[];
    mcpServers?: McpServerConfig[];
    opacity?: number;
    chatModel?: string;
    chatModelProvider?: string;
    multimodalModel?: string;
    multimodalModelProvider?: string;
    multimodalSplitAnalysis?: boolean;
    largeContextEnabled?: boolean;
    reasoningEffort?: ReasoningEffort;
    reasoningLanguage?: ReasoningLanguage;
    passToolReasoning?: boolean;
    showReasoning?: boolean;
    multiModelCollaboration?: boolean;
    collaborationModels?: string[];
    zoom?: number;
    primaryHotkey?: string;
    secondaryHotkey?: string;
    customProviders?: CustomProviderConfig[];
    pixpinPinAiEnabled?: boolean;
    snipastePinAiEnabled?: boolean;
}

export interface ModelSelection {
    id: string;
    provider: string;
}

export interface SelectOption<T extends string> {
    value: T;
    label: Partial<Record<AppLanguage, string>> & Pick<Record<AppLanguage, string>, "en-US">;
}

export function localizedOptionLabel<T extends string>(option: SelectOption<T>, language: AppLanguage) {
    return option.label[language] ?? option.label["en-US"];
}

export const colorSchemeOptions: SelectOption<ColorScheme>[] = [
    {
        value: "dark",
        label: { "zh-CN": "深色", "en-US": "Dark" },
    },
    {
        value: "light",
        label: { "zh-CN": "浅色", "en-US": "Light" },
    },
];

export const languageOptions: SelectOption<AppLanguage>[] = [
    {
        value: "zh-CN",
        label: { "zh-CN": "简体中文", "en-US": "Simplified Chinese", "ja-JP": "簡体字中国語", "ru-RU": "Китайский (упрощенный)", "de-DE": "Chinesisch (vereinfacht)", "fr-FR": "Chinois simplifié", "ko-KR": "중국어(간체)" },
    },
    {
        value: "en-US",
        label: { "zh-CN": "English", "en-US": "English" },
    },
    { value: "ja-JP", label: { "en-US": "Japanese", "ja-JP": "日本語" } },
    { value: "ru-RU", label: { "en-US": "Russian", "ru-RU": "Русский" } },
    { value: "de-DE", label: { "en-US": "German", "de-DE": "Deutsch" } },
    { value: "fr-FR", label: { "en-US": "French", "fr-FR": "Français" } },
    { value: "ko-KR", label: { "en-US": "Korean", "ko-KR": "한국어" } },
];

export const reasoningEffortOptions: SelectOption<ReasoningEffort>[] = [
    {
        value: "disabled",
        label: { "zh-CN": "关闭思考", "en-US": "Disabled", "ja-JP": "無効", "ru-RU": "Отключено", "de-DE": "Deaktiviert", "fr-FR": "Désactivé", "ko-KR": "사용 안 함" },
    },
    {
        value: "high",
        label: { "zh-CN": "高", "en-US": "High", "ja-JP": "高", "ru-RU": "Высокая", "de-DE": "Hoch", "fr-FR": "Élevé", "ko-KR": "높음" },
    },
    {
        value: "max",
        label: { "zh-CN": "最高", "en-US": "Max", "ja-JP": "最大", "ru-RU": "Максимальная", "de-DE": "Maximal", "fr-FR": "Maximum", "ko-KR": "최대" },
    },
];

export const reasoningLanguageOptions: SelectOption<ReasoningLanguage>[] = [
    {
        value: "auto",
        label: { "zh-CN": "自动", "en-US": "Auto", "ja-JP": "自動", "ru-RU": "Авто", "de-DE": "Automatisch", "fr-FR": "Automatique", "ko-KR": "자동" },
    },
    {
        value: "zh",
        label: { "zh-CN": "中文", "en-US": "Chinese", "ja-JP": "中国語", "ru-RU": "Китайский", "de-DE": "Chinesisch", "fr-FR": "Chinois", "ko-KR": "중국어" },
    },
    {
        value: "en",
        label: { "zh-CN": "English", "en-US": "English" },
    },
];

export const webSearchProviderOptions: SelectOption<WebSearchProvider>[] = [
    {
        value: "serper",
        label: { "zh-CN": "Serper", "en-US": "Serper" },
    },
    {
        value: "tavily",
        label: { "zh-CN": "Tavily", "en-US": "Tavily" },
    },
];

export const toolApprovalModeOptions: SelectOption<ToolApprovalMode>[] = [
    {
        value: "ask",
        label: { "zh-CN": "询问", "en-US": "Ask" },
    },
    {
        value: "auto",
        label: { "zh-CN": "自动", "en-US": "Auto" },
    },
    {
        value: "alwaysAllow",
        label: { "zh-CN": "一律允许", "en-US": "Always allow" },
    },
];

export const zoomOptions: SelectOption<string>[] = [
    {
        value: "80",
        label: { "zh-CN": "80%", "en-US": "80%" },
    },
    {
        value: "90",
        label: { "zh-CN": "90%", "en-US": "90%" },
    },
    {
        value: "100",
        label: { "zh-CN": "100% (默认)", "en-US": "100% (Default)", "ja-JP": "100%（既定）", "ru-RU": "100% (по умолчанию)", "de-DE": "100% (Standard)", "fr-FR": "100 % (par défaut)", "ko-KR": "100% (기본값)" },
    },
    {
        value: "110",
        label: { "zh-CN": "110%", "en-US": "110%" },
    },
    {
        value: "120",
        label: { "zh-CN": "120%", "en-US": "120%" },
    },
    {
        value: "130",
        label: { "zh-CN": "130%", "en-US": "130%" },
    },
    {
        value: "140",
        label: { "zh-CN": "140%", "en-US": "140%" },
    },
    {
        value: "150",
        label: { "zh-CN": "150%", "en-US": "150%" },
    },
    {
        value: "175",
        label: { "zh-CN": "175%", "en-US": "175%" },
    },
    {
        value: "200",
        label: { "zh-CN": "200%", "en-US": "200%" },
    },
];

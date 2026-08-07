/** OpenAI-compatible custom provider templates for one-click setup. */
export type ProviderPresetId = "mimo" | "zhipu" | "volcengine" | "minimax" | "kimi";

export type ProviderPreset = {
  id: ProviderPresetId;
  name: string;
  baseUrl: string;
  /** Suggested model IDs when remote /v1/models is unavailable. */
  models: string[];
};

export const CUSTOM_PROVIDER_PRESETS: ProviderPreset[] = [
  {
    id: "mimo",
    name: "小米 MiMo",
    baseUrl: "https://api.xiaomimimo.com/v1",
    models: ["mimo-v2.5-pro", "mimo-v2.5"],
  },
  {
    id: "zhipu",
    name: "智谱 GLM",
    baseUrl: "https://open.bigmodel.cn/api/paas/v4",
    models: ["glm-4-flash", "glm-4-plus"],
  },
  {
    id: "volcengine",
    name: "火山方舟",
    baseUrl: "https://ark.cn-beijing.volces.com/api/v3",
    models: [],
  },
  {
    id: "minimax",
    name: "MiniMax",
    baseUrl: "https://api.minimaxi.com/v1",
    models: ["MiniMax-M3", "MiniMax-Text-01"],
  },
  {
    id: "kimi",
    name: "Kimi",
    baseUrl: "https://api.moonshot.cn/v1",
    models: ["kimi-k2.5", "moonshot-v1-128k", "moonshot-v1-32k"],
  },
];

export function looksLikeHttpUrl(value: string): boolean {
  return /^https?:\/\//i.test(value.trim());
}

export function isCustomProviderConfigured(provider: { baseUrl: string; apiKey: string }): boolean {
  return looksLikeHttpUrl(provider.baseUrl) && provider.apiKey.trim().length > 0;
}

export function parseProviderModels(raw: string): string[] {
  const seen = new Set<string>();
  const models: string[] = [];
  for (const part of raw.split(/[,，\n]/)) {
    const id = part.trim();
    if (!id || seen.has(id)) continue;
    seen.add(id);
    models.push(id);
  }
  return models;
}

export function serializeProviderModels(models: string[]): string {
  return models.join("\n");
}

export function findPresetById(id?: string | null): ProviderPreset | undefined {
  if (!id) return undefined;
  return CUSTOM_PROVIDER_PRESETS.find((preset) => preset.id === id);
}

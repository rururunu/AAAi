import type { Component } from "vue";
import DeepSeekIcon from "@/components/icons/DeepSeekIcon.vue";
import GeminiIcon from "@/components/icons/GeminiIcon.vue";
import type { ChatModelInfo } from "@/types/chat";

export const DEEPSEEK_PROVIDER = "deepseek";
export const GEMINI_PROVIDER = "gemini";

const providerIcons: Record<string, Component> = {
  [DEEPSEEK_PROVIDER]: DeepSeekIcon,
  [GEMINI_PROVIDER]: GeminiIcon,
};

export function getProviderIcon(provider?: string | null): Component | null {
  if (!provider) return null;
  return providerIcons[provider] ?? null;
}

export function isDeepSeekProvider(provider?: string | null): boolean {
  return provider === DEEPSEEK_PROVIDER;
}

export function isGeminiProvider(provider?: string | null): boolean {
  return provider === GEMINI_PROVIDER;
}

/** Human-readable vendor label for model picker section headers. */
export function getProviderDisplayName(provider?: string | null): string {
  const key = provider?.trim().toLowerCase() ?? "";
  if (!key) return "Other";
  if (key === DEEPSEEK_PROVIDER) return "DeepSeek";
  if (key === GEMINI_PROVIDER) return "Gemini";
  return key
    .split(/[-_\s]+/)
    .filter(Boolean)
    .map(capitalizeWord)
    .join(" ");
}

export type ModelProviderGroup = {
  provider: string;
  label: string;
  models: ChatModelInfo[];
};

const PROVIDER_SORT_ORDER = [DEEPSEEK_PROVIDER, GEMINI_PROVIDER];

/** Group models by provider; known vendors first, then A–Z. */
export function groupModelsByProvider(
  models: ChatModelInfo[],
): ModelProviderGroup[] {
  const map = new Map<string, ChatModelInfo[]>();
  for (const model of models) {
    const key = model.provider?.trim() || "other";
    const list = map.get(key);
    if (list) {
      list.push(model);
    } else {
      map.set(key, [model]);
    }
  }

  return Array.from(map.entries())
    .sort(([a], [b]) => {
      const ai = PROVIDER_SORT_ORDER.indexOf(a);
      const bi = PROVIDER_SORT_ORDER.indexOf(b);
      if (ai !== -1 || bi !== -1) {
        if (ai === -1) return 1;
        if (bi === -1) return -1;
        return ai - bi;
      }
      return getProviderDisplayName(a).localeCompare(
        getProviderDisplayName(b),
        undefined,
        { sensitivity: "base" },
      );
    })
    .map(([provider, grouped]) => ({
      provider,
      label: getProviderDisplayName(provider === "other" ? "" : provider),
      models: grouped,
    }));
}

function capitalizeWord(value: string): string {
  if (!value) return value;
  return value.charAt(0).toUpperCase() + value.slice(1);
}

/** Turn Antigravity / Gemini internal ids into readable labels. */
export function formatGeminiDisplayName(modelId: string): string {
  const raw = modelId.trim();
  const rest = raw.replace(/^gemini[-_]/i, "");
  const lower = rest.toLowerCase();

  let tier = "";
  let body = rest;
  if (lower.endsWith("-agent")) {
    tier = "Agent";
    body = rest.slice(0, -"-agent".length);
  } else if (lower.endsWith("-high")) {
    tier = "High";
    body = rest.slice(0, -"-high".length);
  } else if (lower.endsWith("-low")) {
    tier = "Low";
    body = rest.slice(0, -"-low".length);
  }
  body = body.replace(/-+$/, "");

  const parts = body.split("-").filter(Boolean);
  if (parts.length >= 2) {
    const version = parts[0];
    const family = parts.slice(1).map(capitalizeWord).join(" ");
    const base = `Gemini ${version} ${family}`;
    return tier ? `${base} (${tier})` : base;
  }
  if (parts.length === 1) {
    return `Gemini ${capitalizeWord(parts[0])}`;
  }
  return `Gemini ${body}`;
}

/**
 * Short display label for a model. DeepSeek models drop the `deepseek-` prefix
 * (e.g. `deepseek-v4-pro` → `v4-pro`) since the brand icon already conveys the vendor.
 */
export function formatModelDisplayName(
  modelId: string,
  provider?: string | null,
): string {
  const id = modelId.trim();
  if (!id) return id;
  if (isDeepSeekProvider(provider) && /^deepseek[-_]/i.test(id)) {
    return id.replace(/^deepseek[-_]/i, "");
  }
  if (isGeminiProvider(provider)) {
    return formatGeminiDisplayName(id);
  }
  return id;
}

/** Primary label for model pickers and the chat input badge. */
export function getModelDisplayLabel(
  model: Pick<ChatModelInfo, "id" | "provider" | "displayName">,
): string {
  const fromApi = model.displayName?.trim();
  if (fromApi) return fromApi;
  return formatModelDisplayName(model.id, model.provider);
}

/** Optional muted subtitle under the primary model label. */
export function getModelDisplaySubtitle(
  model: Pick<ChatModelInfo, "id" | "provider" | "ownedBy" | "displayName">,
): string | null {
  if (isDeepSeekProvider(model.provider)) {
    return null;
  }
  if (isGeminiProvider(model.provider)) {
    const label = getModelDisplayLabel(model);
    const shortId = formatModelDisplayName(model.id, model.provider);
    if (shortId !== label && shortId !== model.id) {
      return shortId;
    }
    return null;
  }
  const owner = model.ownedBy?.trim();
  return owner || null;
}

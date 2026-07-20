import type { Component } from "vue";
import DeepSeekIcon from "@/components/icons/DeepSeekIcon.vue";

export const DEEPSEEK_PROVIDER = "deepseek";

const providerIcons: Record<string, Component> = {
  [DEEPSEEK_PROVIDER]: DeepSeekIcon,
};

/** Resolve a provider key to its brand icon component, if any. */
export function getProviderIcon(provider?: string | null): Component | null {
  if (!provider) return null;
  return providerIcons[provider] ?? null;
}

export function isDeepSeekProvider(provider?: string | null): boolean {
  return provider === DEEPSEEK_PROVIDER;
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
  return id;
}

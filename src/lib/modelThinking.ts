import type { ChatModelInfo, ModelThinkingVariant } from "@/types/chat";
import type { AppLanguage } from "@/types/setting";
import { tr } from "@/services/i18n";

export function modelHasThinkingVariants(
  model: Pick<ChatModelInfo, "thinkingVariants">,
): boolean {
  return (model.thinkingVariants?.length ?? 0) > 1;
}

export function findModelEntry(
  models: ChatModelInfo[],
  variantOrDefaultId: string,
): ChatModelInfo | undefined {
  const id = variantOrDefaultId.trim();
  if (!id) return undefined;

  return models.find(
    (model) =>
      model.id === id ||
      model.thinkingVariants?.some((variant) => variant.id === id),
  );
}

export function isModelEntrySelected(
  entry: ChatModelInfo,
  variantOrDefaultId: string,
): boolean {
  const id = variantOrDefaultId.trim();
  if (!id) return false;
  if (entry.id === id) return true;
  return entry.thinkingVariants?.some((variant) => variant.id === id) ?? false;
}

export function getActiveThinkingVariant(
  entry: ChatModelInfo,
  variantOrDefaultId: string,
): ModelThinkingVariant | null {
  if (!modelHasThinkingVariants(entry)) {
    return null;
  }

  const id = variantOrDefaultId.trim();
  return (
    entry.thinkingVariants?.find((variant) => variant.id === id) ??
    entry.thinkingVariants?.find((variant) => variant.id === entry.id) ??
    entry.thinkingVariants?.[0] ??
    null
  );
}

export function getThinkingTierOptions(
  entry: ChatModelInfo,
): ModelThinkingVariant[] {
  return entry.thinkingVariants ?? [];
}

export function localizeThinkingTierLabel(
  label: string,
  language: AppLanguage,
): string {
  const normalized = label.trim().toLowerCase();
  if (normalized === "low") return tr(language, "thinkingTierLow");
  if (normalized === "high") return tr(language, "thinkingTierHigh");
  if (normalized === "agent") return tr(language, "thinkingTierAgent");
  if (normalized === "default") return tr(language, "thinkingTierDefault");
  return label;
}

export function isKnownModelSelection(
  models: ChatModelInfo[],
  variantOrDefaultId: string,
): boolean {
  return !!findModelEntry(models, variantOrDefaultId);
}

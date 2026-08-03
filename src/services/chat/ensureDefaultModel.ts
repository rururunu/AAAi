import { findModelEntry, isKnownModelSelection } from "@/lib/modelThinking";
import { useChatModelStore } from "@/stores/chatModel";
import { useSettingStore } from "@/stores/setting";
import type { ChatModelInfo } from "@/types/chat";

/** Pick the first available model when none (or an invalid one) is selected. */
export async function ensureDefaultChatModel(
  options: { refresh?: boolean } = {},
): Promise<ChatModelInfo | null> {
  const chatModelStore = useChatModelStore();
  const settingStore = useSettingStore();

  if (options.refresh) {
    await chatModelStore.refresh();
  } else if (chatModelStore.models.length === 0 && !chatModelStore.loading) {
    await chatModelStore.fetch();
  }

  const models = chatModelStore.models;
  if (models.length === 0) return null;

  const current = settingStore.chatModel.trim();
  if (
    current &&
    isKnownModelSelection(models, current, settingStore.chatModelProvider)
  ) {
    return (
      findModelEntry(models, current, settingStore.chatModelProvider) ?? models[0]!
    );
  }

  const first = models[0]!;
  await settingStore.update({
    chatModel: first.id,
    chatModelProvider: first.provider,
  });
  return first;
}

export const CONFIGURE_PROVIDER_MARKER = "<!--peek:configure-provider-->";

export function isConfigureProviderError(content: string): boolean {
  if (!content.trim()) return false;
  if (content.includes(CONFIGURE_PROVIDER_MARKER)) return true;
  return /No model selected|credentials are not configured|Model credentials are not configured|Sign in to Gemini in Settings/i.test(
    content,
  );
}

export function stripConfigureProviderMarker(content: string): string {
  return content.replace(CONFIGURE_PROVIDER_MARKER, "").trim();
}

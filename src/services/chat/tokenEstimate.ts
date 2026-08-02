import type { ChatMessage } from "@/types/chat";

const CHARS_PER_TOKEN = 4;
const IMAGE_TOKEN_ESTIMATE = 1000;

export function estimateTextTokens(text: string | undefined): number {
  if (!text) return 0;

  const imageCount = text.match(/data:image\//g)?.length ?? 0;
  const countableText = imageCount > 0
    ? text.replace(/data:image\/[^)]+/g, "image_placeholder")
    : text;
  const characters = [...countableText].length;
  const textTokens = characters > 0 ? Math.max(1, Math.floor(characters / CHARS_PER_TOKEN)) : 0;
  return textTokens + imageCount * IMAGE_TOKEN_ESTIMATE;
}

export function estimateMessageTokens(message: ChatMessage): number {
  if (message.estimatedTokens != null && Number.isFinite(message.estimatedTokens)) {
    return message.estimatedTokens;
  }

  let total = estimateTextTokens(message.content) + estimateTextTokens(message.reasoning);

  for (const activity of message.toolActivities ?? []) {
    total += estimateTextTokens(activity.toolName);
    total += estimateTextTokens(activity.title);
    total += estimateTextTokens(activity.detail);
    total += estimateTextTokens(activity.result);
    if (activity.arguments) total += estimateTextTokens(JSON.stringify(activity.arguments));
  }

  return total + 4;
}

export function formatTokenCount(tokens: number, language?: string): string {
  return new Intl.NumberFormat(language, {
    notation: "compact",
    maximumFractionDigits: 1,
  }).format(tokens);
}

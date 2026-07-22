/** Persisted marker so soft-injects survive history reload. */
export const SOFT_INJECT_MARKER = "<!--peek:soft-inject-->\n";

export function markSoftInjectContent(content: string): string {
  const trimmed = content.trim();
  if (!trimmed || trimmed.startsWith(SOFT_INJECT_MARKER.trim())) {
    return trimmed;
  }
  return `${SOFT_INJECT_MARKER}${trimmed}`;
}

export function isSoftInjectContent(content: string): boolean {
  return content.trimStart().startsWith("<!--peek:soft-inject-->");
}

export function stripSoftInjectMarker(content: string): string {
  const trimmed = content.trimStart();
  if (trimmed.startsWith("<!--peek:soft-inject-->")) {
    return trimmed.replace(/^<!--peek:soft-inject-->\s*/, "");
  }
  return content;
}

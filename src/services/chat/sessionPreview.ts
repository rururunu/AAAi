/**
 * Format session preview / titlebar text so @/# tokens read like composer chips
 * (basename / short id) instead of raw paths leaking into chrome.
 */

const INLINE_TOKEN_RE = /@(?:"([^"]+)"|([^\s@#]+))|#(?:skill|mcp):([A-Za-z0-9_.-]+)/g;
const LEADING_CHIP_RE =
  /^(?:@(?:"[^"]+"|[^\s@#]+)|#(?:skill|mcp):[A-Za-z0-9_.-]+|#\S+)(?:\s+(?:@(?:"[^"]+"|[^\s@#]+)|#(?:skill|mcp):[A-Za-z0-9_.-]+|#\S+))*\s*/;

function fileBaseName(path: string): string {
  return path.split(/[/\\]/).pop() || path;
}

function prettifyTokens(text: string): string {
  return text.replace(
    INLINE_TOKEN_RE,
    (_match, quoted: string | undefined, bare: string | undefined, hashId: string | undefined) => {
      if (hashId) return `#${hashId}`;
      return `@${fileBaseName(quoted || bare || "")}`;
    },
  );
}

function truncateChars(value: string, max: number): string {
  if ([...value].length <= max) return value;
  return `${[...value].slice(0, max).join("")}…`;
}

/** Human-facing session title for titlebar / sidebar lists. */
export function formatSessionPreview(preview: string, maxLen = 48): string {
  const normalized = preview.replace(/\s+/g, " ").trim();
  if (!normalized) return "";

  const pretty = prettifyTokens(normalized);
  const prose = pretty.replace(LEADING_CHIP_RE, "").trim();
  return truncateChars(prose || pretty, maxLen);
}

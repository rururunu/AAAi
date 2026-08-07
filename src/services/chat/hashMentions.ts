/**
 * `#` resource mentions for Skills and MCP servers.
 *
 * Wire format in the sent message (human-readable + agent-parseable):
 * - `#skill:generate_bid_tech`
 * - `#mcp:server-id`
 *
 * Chips in the composer use the same tokens via {@link formatHashMention}.
 */

export type HashResourceKind = "skill" | "mcp";

export type HashMentionItem = {
  kind: HashResourceKind;
  /** Skill name or MCP server id. */
  id: string;
  /** Short UI label. */
  title: string;
  /** Optional one-line description. */
  description?: string;
  /** Remote or local icon URL (Smithery / cached install icon). */
  iconUrl?: string | null;
  /** Vendor / registry identity, e.g. `gmail` or `adamamer20/paper-search-mcp-openai`. */
  vendor?: string;
};

/** Token written into the user message. */
export function formatHashMention(kind: HashResourceKind, id: string): string {
  const cleaned = id.trim().replace(/\s+/g, "-");
  return `#${kind}:${cleaned}`;
}

const HASH_TOKEN_RE = /#(skill|mcp):([A-Za-z0-9_.-]+)/g;

/** Parse all `#skill:` / `#mcp:` tokens from free text. */
export function parseHashMentions(text: string): Array<{ kind: HashResourceKind; id: string }> {
  const out: Array<{ kind: HashResourceKind; id: string }> = [];
  const re = new RegExp(HASH_TOKEN_RE.source, "g");
  let match: RegExpExecArray | null;
  while ((match = re.exec(text)) !== null) {
    const kind = match[1] as HashResourceKind;
    const id = match[2] ?? "";
    if (id) out.push({ kind, id });
  }
  return out;
}

/** Active `#…` query at the caret end of the live input (mirrors `@` file mentions). */
export function activeHashMention(message: string): { query: string; start: number } | null {
  // Allow `#`, `#sk`, `#skill:`, `#mcp:foo`
  const match = message.match(/(?:^|\s)#([^\s]*)$/);
  if (!match || match.index === undefined) return null;
  const hashIndex = match.index + match[0].indexOf("#");
  return { query: match[1] ?? "", start: hashIndex };
}

/**
 * Filter skill/MCP catalog by the typed query after `#`.
 * Supports prefixes like `skill:`, `mcp:`, or free-text against id/title/vendor/desc.
 */
export function filterHashMentionItems(
  items: readonly HashMentionItem[],
  rawQuery: string,
): HashMentionItem[] {
  const query = rawQuery.trim().toLowerCase();
  if (!query) return items.slice(0, 24);

  let kindFilter: HashResourceKind | null = null;
  let needle = query;

  if (query === "skill" || query === "skills") {
    kindFilter = "skill";
    needle = "";
  } else if (query === "mcp") {
    kindFilter = "mcp";
    needle = "";
  } else if (query.startsWith("skill:") || query.startsWith("skill/")) {
    kindFilter = "skill";
    needle = query.slice(6);
  } else if (query.startsWith("mcp:") || query.startsWith("mcp/")) {
    kindFilter = "mcp";
    needle = query.slice(4);
  }

  return items
    .filter((item) => {
      if (kindFilter && item.kind !== kindFilter) return false;
      if (!needle) return true;
      const hay =
        `${item.id} ${item.title} ${item.vendor ?? ""} ${item.description ?? ""}`.toLowerCase();
      return hay.includes(needle);
    })
    .sort((left, right) => {
      const leftId = left.id.toLowerCase();
      const rightId = right.id.toLowerCase();
      const leftVendor = (left.vendor ?? "").toLowerCase();
      const rightVendor = (right.vendor ?? "").toLowerCase();
      const leftRank =
        needle && (leftId.startsWith(needle) || leftVendor.startsWith(needle)) ? 0 : 1;
      const rightRank =
        needle && (rightId.startsWith(needle) || rightVendor.startsWith(needle)) ? 0 : 1;
      if (leftRank !== rightRank) return leftRank - rightRank;
      if (left.kind !== right.kind) return left.kind === "skill" ? -1 : 1;
      return leftId.localeCompare(rightId);
    })
    .slice(0, 24);
}

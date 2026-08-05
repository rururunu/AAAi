/**
 * Composer chip / segment model used by the chat input bar.
 *
 * Segments sit ahead of the live textarea value and preserve mid-sentence
 * order for mentions, pasted blocks, and selection chips.
 */

export type ComposerSegment =
  | { kind: "text"; text: string }
  | { kind: "mention"; path: string }
  | { kind: "paste"; text: string }
  | { kind: "selection"; lines: number };

/** Count lines in pasted text (treats empty as zero). */
export function pasteLineCount(text: string): number {
  return text ? text.split(/\r\n|\r|\n/).length : 0;
}

/** Serialize a file path as an @-mention, quoting when it contains spaces. */
export function formatMentionPath(path: string): string {
  return /\s/.test(path) ? `@"${path}"` : `@${path}`;
}

/**
 * Join composer parts while preserving mid-sentence tag order
 * (no forced blank lines between adjacent chips/text).
 */
export function joinInlineParts(parts: string[]): string {
  let out = "";
  for (const part of parts) {
    if (!part) continue;
    if (!out) {
      out = part;
      continue;
    }
    if (/\s$/.test(out) || /^\s/.test(part)) {
      out += part;
    } else {
      out += ` ${part}`;
    }
  }
  return out;
}

/**
 * Flatten frozen segments plus the live textarea into a single sendable string.
 * Selection chips are omitted — the parent attaches selection separately.
 */
export function serializeComposerSegments(
  segments: readonly ComposerSegment[],
  liveMessage: string,
): string {
  const parts: string[] = [];
  for (const seg of segments) {
    if (seg.kind === "text") {
      parts.push(seg.text);
    } else if (seg.kind === "mention") {
      parts.push(formatMentionPath(seg.path));
    } else if (seg.kind === "paste") {
      parts.push(seg.text);
    }
  }
  if (liveMessage) {
    parts.push(liveMessage);
  }
  return joinInlineParts(parts);
}

/**
 * Append a segment, merging adjacent text/paste chips when possible.
 * Returns the next segment list (immutable-friendly for callers that prefer it).
 */
export function appendComposerSegment(
  segments: ComposerSegment[],
  segment: ComposerSegment,
): ComposerSegment[] {
  if (segment.kind === "text") {
    if (!segment.text) return segments;
    const last = segments[segments.length - 1];
    if (last?.kind === "text") {
      last.text = joinInlineParts([last.text, segment.text]);
      return segments;
    }
  }
  if (segment.kind === "paste") {
    const last = segments[segments.length - 1];
    if (last?.kind === "paste") {
      last.text = `${last.text}\n${segment.text}`;
      return segments;
    }
  }
  segments.push(segment);
  return segments;
}

/**
 * Move trailing typed text into a frozen text segment so a new chip can sit after it.
 * Mutates `segments` and clears `liveMessage` via the returned value.
 */
export function flushLiveMessageToSegments(
  segments: ComposerSegment[],
  liveMessage: string,
): { segments: ComposerSegment[]; liveMessage: string } {
  if (!liveMessage) {
    return { segments, liveMessage: "" };
  }
  const last = segments[segments.length - 1];
  if (last?.kind === "text") {
    last.text = joinInlineParts([last.text, liveMessage]);
  } else {
    segments.push({ kind: "text", text: liveMessage });
  }
  return { segments, liveMessage: "" };
}

const SELECTION_OPEN = /\n\n<peek-selection lines="(\d+)">\n/;
const SELECTION_CLOSE = "\n</peek-selection>";

export interface SelectionAttachment {
  message: string;
  selection?: string;
  lineCount?: number;
}

export function selectionLineCount(selection: string) {
  const normalized = selection.trim();
  return normalized ? normalized.split(/\r\n|\r|\n/).length : 0;
}

export function attachSelection(message: string, selection?: string) {
  const normalized = selection?.trim() ?? "";
  if (!normalized) return message.trim();
  const lines = selectionLineCount(normalized);
  return `${message.trim()}\n\n<peek-selection lines="${lines}">\n${normalized}${SELECTION_CLOSE}`;
}

export function parseSelectionAttachment(content: string): SelectionAttachment {
  const match = SELECTION_OPEN.exec(content);
  if (!match) return { message: content };

  const closeIndex = content.lastIndexOf(SELECTION_CLOSE);
  if (closeIndex < match.index + match[0].length) return { message: content };

  return {
    message: content.slice(0, match.index).trim(),
    selection: content.slice(match.index + match[0].length, closeIndex),
    lineCount: Number(match[1]),
  };
}

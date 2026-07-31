import { ipcInvoke } from "@/services/ipc/commands";
import { IPC_COMMANDS } from "@/types/ipc";

export type CodeDiffLineKind = "context" | "addition" | "deletion";

export interface CodeDiffLine {
  lineNumber: number;
  text: string;
  kind: CodeDiffLineKind;
}

export interface CodeDiffRow {
  left: CodeDiffLine | null;
  right: CodeDiffLine | null;
}

export interface CodeDiffDocument {
  rows: CodeDiffRow[];
}

export interface CodeDiffRequest {
  oldText?: string | null;
  newText?: string | null;
  unifiedDiff: string;
}

export async function buildCodeDiff(request: CodeDiffRequest): Promise<CodeDiffDocument> {
  try {
    const document = await ipcInvoke<CodeDiffDocument>(IPC_COMMANDS.buildCodeDiff, { request });
    if (document.rows.length || !request.unifiedDiff.trim()) {
      return document;
    }
  } catch (error) {
    console.warn("Rust code diff unavailable; rendering unified diff locally.", error);
  }

  // The native command is the primary implementation. This fallback keeps
  // completed changes visible while a dev backend is rebuilding or a legacy
  // window has not loaded the new command capability yet.
  return parseUnifiedDiff(request.unifiedDiff);
}

function parseUnifiedDiff(diff: string): CodeDiffDocument {
  const rows: CodeDiffRow[] = [];
  const deletions: CodeDiffLine[] = [];
  const additions: CodeDiffLine[] = [];
  let oldLine = 0;
  let newLine = 0;

  const flushChanges = () => {
    const count = Math.max(deletions.length, additions.length);
    for (let index = 0; index < count; index += 1) {
      rows.push({ left: deletions[index] ?? null, right: additions[index] ?? null });
    }
    deletions.length = 0;
    additions.length = 0;
  };

  for (const raw of diff.replace(/\r\n/g, "\n").split("\n")) {
    const hunk = raw.match(/^@@\s+-(\d+)(?:,\d+)?\s+\+(\d+)(?:,\d+)?\s+@@/);
    if (hunk) {
      flushChanges();
      oldLine = Number(hunk[1]);
      newLine = Number(hunk[2]);
      continue;
    }
    if (raw.startsWith("--- ") || raw.startsWith("+++ ") || raw.startsWith("diff ") || raw.startsWith("index ")) {
      continue;
    }
    if (raw.startsWith("-")) {
      deletions.push({ lineNumber: oldLine, text: raw.slice(1), kind: "deletion" });
      oldLine += 1;
      continue;
    }
    if (raw.startsWith("+")) {
      additions.push({ lineNumber: newLine, text: raw.slice(1), kind: "addition" });
      newLine += 1;
      continue;
    }
    if (raw.startsWith(" ")) {
      flushChanges();
      const text = raw.slice(1);
      rows.push({
        left: { lineNumber: oldLine, text, kind: "context" },
        right: { lineNumber: newLine, text, kind: "context" },
      });
      oldLine += 1;
      newLine += 1;
    }
  }

  flushChanges();
  return { rows };
}

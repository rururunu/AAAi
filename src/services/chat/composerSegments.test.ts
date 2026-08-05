import { describe, expect, it } from "vitest";
import {
  appendComposerSegment,
  flushLiveMessageToSegments,
  formatMentionPath,
  joinInlineParts,
  pasteLineCount,
  serializeComposerSegments,
  type ComposerSegment,
} from "./composerSegments";

describe("composerSegments", () => {
  it("counts paste lines", () => {
    expect(pasteLineCount("")).toBe(0);
    expect(pasteLineCount("a\nb\nc")).toBe(3);
  });

  it("quotes mention paths that contain spaces", () => {
    expect(formatMentionPath("src/a.ts")).toBe("@src/a.ts");
    expect(formatMentionPath("my file.ts")).toBe('@"my file.ts"');
  });

  it("serializes segments ahead of live text without forced blank lines", () => {
    const segments: ComposerSegment[] = [
      { kind: "text", text: "see " },
      { kind: "mention", path: "src/a.ts" },
    ];
    expect(serializeComposerSegments(segments, " please")).toBe("see @src/a.ts please");
  });

  it("merges adjacent text and paste chips", () => {
    const segments: ComposerSegment[] = [];
    appendComposerSegment(segments, { kind: "text", text: "hello" });
    appendComposerSegment(segments, { kind: "text", text: " world" });
    appendComposerSegment(segments, { kind: "paste", text: "p1" });
    appendComposerSegment(segments, { kind: "paste", text: "p2" });
    expect(segments).toEqual([
      { kind: "text", text: "hello world" },
      { kind: "paste", text: "p1\np2" },
    ]);
  });

  it("flushes live message into a trailing text segment", () => {
    const segments: ComposerSegment[] = [{ kind: "mention", path: "a.ts" }];
    const next = flushLiveMessageToSegments(segments, " note");
    expect(next.liveMessage).toBe("");
    expect(next.segments).toEqual([
      { kind: "mention", path: "a.ts" },
      { kind: "text", text: " note" },
    ]);
  });

  it("joins inline parts with a single separating space when needed", () => {
    expect(joinInlineParts(["a", "b"])).toBe("a b");
    expect(joinInlineParts(["a ", "b"])).toBe("a b");
  });
});

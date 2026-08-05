import { describe, expect, it } from "vitest";
import {
  normalizeChatStarted,
  normalizeMessage,
  normalizeRole,
  normalizeStatus,
  normalizeToolActivityEvent,
  resolveSessionId,
} from "./normalize";

describe("resolveSessionId", () => {
  it("returns the first non-empty candidate", () => {
    expect(resolveSessionId(undefined, "  ", "abc", "def")).toBe("abc");
  });

  it("returns empty string when nothing is usable", () => {
    expect(resolveSessionId(undefined, "", "   ")).toBe("");
  });
});

describe("normalizeRole / normalizeStatus", () => {
  it("normalizes known roles and falls back to assistant", () => {
    expect(normalizeRole("USER")).toBe("user");
    expect(normalizeRole("tool")).toBe("tool");
    expect(normalizeRole("nope")).toBe("assistant");
  });

  it("normalizes known statuses and falls back to done", () => {
    expect(normalizeStatus("Streaming")).toBe("streaming");
    expect(normalizeStatus("cancelled")).toBe("cancelled");
    expect(normalizeStatus("weird")).toBe("done");
  });
});

describe("normalizeMessage", () => {
  it("returns null without id or session", () => {
    expect(normalizeMessage({ content: "x" })).toBeNull();
    expect(normalizeMessage({ id: "m1" })).toBeNull();
  });

  it("fills defaults from camelCase payload", () => {
    const message = normalizeMessage({
      id: "m1",
      sessionId: "s1",
      role: "user",
      content: "hi",
    });
    expect(message).toMatchObject({
      id: "m1",
      sessionId: "s1",
      role: "user",
      content: "hi",
      status: "done",
      injected: false,
    });
    expect(typeof message?.timestamp).toBe("number");
  });
});

describe("normalizeChatStarted", () => {
  it("accepts snake_case session and message fields", () => {
    const started = normalizeChatStarted({
      session_id: "s1",
      user_message: { id: "u1", role: "user", content: "q" },
      assistant_message: { id: "a1", role: "assistant", content: "" },
    });
    expect(started?.sessionId).toBe("s1");
    expect(started?.userMessage.id).toBe("u1");
    expect(started?.assistantMessage.id).toBe("a1");
  });

  it("returns null when messages are incomplete", () => {
    expect(
      normalizeChatStarted({
        sessionId: "s1",
        userMessage: { id: "u1", content: "q" },
      }),
    ).toBeNull();
  });
});

describe("normalizeToolActivityEvent", () => {
  it("requires session, message, and activity ids", () => {
    expect(
      normalizeToolActivityEvent({
        sessionId: "s1",
        messageId: "m1",
      }),
    ).toBeNull();
  });

  it("normalizes snake_case tool activity", () => {
    const event = normalizeToolActivityEvent({
      session_id: "s1",
      message_id: "m1",
      activity_id: "t1",
      tool_name: "Read",
      status: "running",
    });
    expect(event).toMatchObject({
      sessionId: "s1",
      messageId: "m1",
      activity: {
        id: "t1",
        toolName: "Read",
        status: "running",
        success: true,
      },
    });
  });
});

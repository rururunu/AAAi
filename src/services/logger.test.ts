import { describe, expect, it } from "vitest";
import { clearRecentLogs, createLogger, getRecentLogs } from "./logger";

describe("createLogger", () => {
  it("records entries into the ring buffer", () => {
    clearRecentLogs();
    const log = createLogger("test-scope");
    log.warn("hello", { n: 1 });
    log.error("boom");

    const recent = getRecentLogs();
    expect(recent.length).toBeGreaterThanOrEqual(2);
    const last = recent.at(-1);
    expect(last?.scope).toBe("test-scope");
    expect(last?.level).toBe("error");
    expect(last?.message).toBe("boom");
  });
});

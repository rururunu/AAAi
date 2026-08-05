import { afterEach, describe, expect, it, vi } from "vitest";
import { createRafBatch } from "./rafBatch";

describe("createRafBatch", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("batches pushes and flushes on drain without rAF", async () => {
    vi.stubGlobal("requestAnimationFrame", undefined);
    vi.stubGlobal("cancelAnimationFrame", undefined);

    const flush = vi.fn();
    const batch = createRafBatch<number>(flush);

    batch.push(1);
    batch.push(2);
    expect(batch.size()).toBe(2);

    await Promise.resolve();
    expect(flush).toHaveBeenCalledWith([1, 2]);
    expect(batch.size()).toBe(0);
  });

  it("schedules requestAnimationFrame when available", () => {
    const callbacks: FrameRequestCallback[] = [];
    vi.stubGlobal("requestAnimationFrame", (cb: FrameRequestCallback) => {
      callbacks.push(cb);
      return 42;
    });
    vi.stubGlobal("cancelAnimationFrame", vi.fn());

    const flush = vi.fn();
    const batch = createRafBatch<string>(flush);
    batch.push("a");
    batch.push("b");
    expect(flush).not.toHaveBeenCalled();
    expect(callbacks).toHaveLength(1);

    callbacks[0]!(0);
    expect(flush).toHaveBeenCalledWith(["a", "b"]);
  });

  it("drain cancels pending rAF and flushes immediately", () => {
    const cancel = vi.fn();
    vi.stubGlobal("requestAnimationFrame", () => 7);
    vi.stubGlobal("cancelAnimationFrame", cancel);

    const flush = vi.fn();
    const batch = createRafBatch<number>(flush);
    batch.push(9);
    batch.drain();

    expect(cancel).toHaveBeenCalledWith(7);
    expect(flush).toHaveBeenCalledWith([9]);
  });
});

/**
 * HTML boot splash helpers. The splash lives in index.html and must stay
 * visible until the Vue workbench/overlay loading layer has painted, so the
 * user never sees a blank frame between splash → Suspense → app chrome.
 */

/** Wait two animation frames so layout + paint have settled. */
export function waitForNextPaint(): Promise<void> {
  return new Promise((resolve) => {
    requestAnimationFrame(() => {
      requestAnimationFrame(() => resolve());
    });
  });
}

/**
 * Hide the `#boot-splash` element, optionally fading out.
 * Honors `prefers-reduced-motion`.
 */
export function hideBootSplash(options?: { fadeMs?: number }): void {
  const splash = document.getElementById("boot-splash");
  if (!splash || splash.hasAttribute("hidden")) return;

  const fadeMs = Math.max(0, options?.fadeMs ?? 0);
  if (fadeMs <= 0 || window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
    splash.setAttribute("hidden", "");
    splash.setAttribute("aria-busy", "false");
    return;
  }

  splash.style.transition = `opacity ${fadeMs}ms ease`;
  splash.style.opacity = "0";
  splash.setAttribute("aria-busy", "false");
  window.setTimeout(() => {
    splash.setAttribute("hidden", "");
    splash.style.transition = "";
    splash.style.opacity = "";
  }, fadeMs);
}

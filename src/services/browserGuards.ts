const browserFunctionKeys = new Set([
  "F1",
  "F3",
  "F5",
  "F6",
  "F7",
  "F11",
  "F12",
  "BrowserBack",
  "BrowserForward",
  "BrowserHome",
  "BrowserRefresh",
  "BrowserSearch",
  "BrowserStop",
]);

const browserModifierKeys = new Set([
  "+",
  "-",
  "0",
  "=",
  "b",
  "d",
  "e",
  "f",
  "g",
  "h",
  "j",
  "k",
  "l",
  "n",
  "o",
  "p",
  "r",
  "s",
  "t",
  "u",
  "w",
  "pagedown",
  "pageup",
]);

const guardMarker = "__ANYA_BROWSER_GUARDS_INSTALLED__";

type GuardedWindow = Window & Record<typeof guardMarker, boolean | undefined>;

export function isBrowserShortcut(event: KeyboardEvent): boolean {
  if (browserFunctionKeys.has(event.key)) {
    return true;
  }

  if (event.altKey && ["ArrowLeft", "ArrowRight", "Home"].includes(event.key)) {
    return true;
  }

  const hasCommandModifier = event.ctrlKey || event.metaKey;
  const key = event.key.toLowerCase();
  const opensDeveloperTools =
    hasCommandModifier &&
    ((event.shiftKey && ["c", "i", "j"].includes(key)) ||
      (event.altKey && ["c", "i", "j"].includes(key)));
  if (opensDeveloperTools) {
    return true;
  }

  return hasCommandModifier && browserModifierKeys.has(key);
}

export function installBrowserGuards(target: Window = window): void {
  const guardedWindow = target as GuardedWindow;
  if (guardedWindow[guardMarker]) {
    return;
  }
  guardedWindow[guardMarker] = true;

  target.addEventListener("contextmenu", (event) => event.preventDefault(), {
    capture: true,
  });

  target.addEventListener(
    "keydown",
    (event) => {
      if (isBrowserShortcut(event)) {
        event.preventDefault();
      }
    },
    { capture: true },
  );

  target.addEventListener(
    "wheel",
    (event) => {
      if (event.ctrlKey || event.metaKey) {
        event.preventDefault();
      }
    },
    { capture: true, passive: false },
  );

  target.addEventListener(
    "auxclick",
    (event) => {
      if (event.button === 3 || event.button === 4) {
        event.preventDefault();
      }
    },
    { capture: true },
  );
}

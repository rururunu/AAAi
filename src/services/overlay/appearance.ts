import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

function isPeekWindow(label: string) {
  return label === "overlay" || label.startsWith("overlay-");
}

export async function refreshOverlayWindowBackground() {
  const window = getCurrentWebviewWindow();
  if (!isPeekWindow(window.label)) {
    return;
  }

  try {
    await window.clearEffects();
    await window.setShadow(false);
  } catch (error) {
    console.error("overlay window background failed:", error);
  }
}

export async function applyOpacity(opacity: number) {
  document.documentElement.style.setProperty("--peek-opacity", String(opacity / 100));
  document.documentElement.classList.toggle("frosted-glass", opacity < 100);
  await refreshOverlayWindowBackground();
}

export function markPeekWindow() {
  document.documentElement.classList.add("peek-window");
  void refreshOverlayWindowBackground();
}

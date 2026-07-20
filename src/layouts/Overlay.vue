<template>
  <div class="overlay-shell" data-tauri-drag-region>
    <PeekPanel
      :mode="mode"
      :session-id="sessionId"
      :captured-context="capturedContext"
      @layout-change="handleLayoutChange"
      @enter-chat="enterChatMode"
      @context-consumed="capturedContext = null"
      @selection-removed="removeCapturedSelection"
      @close="close"
    />
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
// trigger rebuild
import { LogicalPosition, LogicalSize } from "@tauri-apps/api/dpi";
import { currentMonitor } from "@tauri-apps/api/window";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import PeekPanel from "@/components/chat/PeekPanel.vue";
import { closeOverlay, setOverlayChatMode, takeOverlayContext } from "@/services/ipc";
import { useChatStore } from "@/stores/chat";
import { useSettingStore } from "@/stores/setting";
import type { CapturedContext } from "@/types/chat";
import { IPC_EVENTS } from "@/types/ipc";

const chatStore = useChatStore();
const settingStore = useSettingStore();

const capturedContext = ref<CapturedContext | null>(null);

function removeCapturedSelection() {
  if (!capturedContext.value) return;
  capturedContext.value = {
    ...capturedContext.value,
    selection: undefined,
  };
}

const PANEL_WIDTH = 640;
const INPUT_HEIGHT = 82;
const OVERLAY_MIN_HEIGHT_INPUT = INPUT_HEIGHT;
const CHAT_HEIGHT = 420;
const OVERLAY_MIN_HEIGHT_CHAT = 240;
const SUGGESTION_ROW_HEIGHT = 30;
const SUGGESTION_PADDING = 9;
const PICKER_META_ROWS = 2;
const PICKER_META_ROW_HEIGHT = 28;
const PICKER_VISIBLE_ROWS = 8;
const CONTEXT_PREVIEW_HEIGHT = 30;
const INPUT_BAR_HEIGHT = INPUT_HEIGHT;

const mode = ref<"input" | "chat">("input");
const sessionId = ref("");
const lastComposerExtraHeight = ref(0);
const chatWindowInitialized = ref(false);
let layoutResizeQueue = Promise.resolve();

// 获取当前窗口的 label，用于所有 IPC 调用
const windowLabel = getCurrentWebviewWindow().label;

function computePickerHeight(rowCount: number) {
  if (rowCount <= 0) {
    return 0;
  }
  const metaHeight = PICKER_META_ROWS * PICKER_META_ROW_HEIGHT;
  const optionRows = Math.max(rowCount - PICKER_META_ROWS, 0);
  const visibleRows = Math.min(optionRows, PICKER_VISIBLE_ROWS);
  return SUGGESTION_PADDING + metaHeight + visibleRows * SUGGESTION_ROW_HEIGHT;
}

async function applySizeConstraints(
  layout: "input" | "chat",
  minHeight: number,
) {
  const window = getCurrentWebviewWindow();
  const zoom = (settingStore.zoom || 100) / 100;

  const panelWidth = PANEL_WIDTH * zoom;
  const height = minHeight * zoom;
  await window.setMinSize(new LogicalSize(panelWidth, height));

  if (layout === "chat") {
    await window.setMaxSize(new LogicalSize(panelWidth, 10000 * zoom));
  } else {
    await window.setMaxSize(null);
  }
}

function waitForNextFrame(count = 2): Promise<void> {
  return new Promise((resolve) => {
    let remaining = count;
    const tick = () => {
      remaining -= 1;
      if (remaining <= 0) {
        resolve();
        return;
      }
      requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);
  });
}

async function adjustWindowHeightBy(deltaDesignHeight: number, resizable: boolean) {
  if (Math.abs(deltaDesignHeight) < 0.5) {
    return;
  }

  const window = getCurrentWebviewWindow();
  const scaleFactor = await window.scaleFactor();
  const physicalPosition = await window.outerPosition();
  const physicalSize = await window.outerSize();

  const logicalPos = physicalPosition.toLogical(scaleFactor);
  const logicalSize = physicalSize.toLogical(scaleFactor);
  const zoom = (settingStore.zoom || 100) / 100;
  const scaledDelta = deltaDesignHeight * zoom;

  await window.setResizable(resizable);
  await window.setMaximizable(false);
  await window.setSize(
    new LogicalSize(logicalSize.width, logicalSize.height + scaledDelta),
  );
  await window.setPosition(
    new LogicalPosition(logicalPos.x, logicalPos.y - scaledDelta),
  );
}

async function resizeWindow(
  width: number,
  height: number,
  resizable: boolean,
  skipPositionCorrection = false,
  verticalAnchor: "top" | "bottom" = "top",
) {
  const window = getCurrentWebviewWindow();
  const scaleFactor = await window.scaleFactor();
  const physicalPosition = await window.outerPosition();
  const physicalSize = await window.outerSize();

  const logicalPos = physicalPosition.toLogical(scaleFactor);
  const logicalSize = physicalSize.toLogical(scaleFactor);

  const zoom = (settingStore.zoom || 100) / 100;
  const scaledWidth = width * zoom;
  const scaledHeight = height * zoom;

  const currentHeight = logicalSize.height;
  const currentWidth = logicalSize.width;

  const delta = scaledHeight - currentHeight;
  const deltaWidth = scaledWidth - currentWidth;

  await window.setResizable(resizable);
  await window.setMaximizable(false);
  await window.setSize(new LogicalSize(scaledWidth, scaledHeight));

  if (!skipPositionCorrection && (Math.abs(delta) > 0.5 || Math.abs(deltaWidth) > 0.5)) {
    // 水平：居中扩展，并 clamp 到当前显示器左右边界
    let newX = logicalPos.x - deltaWidth / 2;

    const monitor = await currentMonitor();
    if (monitor) {
      const monitorPosition = monitor.position.toLogical(scaleFactor);
      const monitorSize = monitor.size.toLogical(scaleFactor);
      const monitorRight = monitorPosition.x + monitorSize.width;
      newX = Math.max(monitorPosition.x, newX);
      newX = Math.min(newX, monitorRight - scaledWidth);
    } else {
      newX = Math.max(0, newX);
    }

    // 输入模式以底边为锚点，建议列表只向上展开，输入框保持原位。
    let newY = verticalAnchor === "bottom"
      ? logicalPos.y - delta
      : logicalPos.y;

    if (verticalAnchor === "top") {
      if (monitor) {
        const monitorPosition = monitor.position.toLogical(scaleFactor);
        const monitorSize = monitor.size.toLogical(scaleFactor);
        const monitorBottom = monitorPosition.y + monitorSize.height;
        if (newY + scaledHeight > monitorBottom) {
          newY = Math.max(monitorPosition.y, monitorBottom - scaledHeight);
        }
      }
    }

    await window.setPosition(new LogicalPosition(newX, newY));
  }

}

function queueLayoutResize(operation: () => Promise<void>) {
  layoutResizeQueue = layoutResizeQueue
    .then(operation)
    .catch((error) => console.error("Failed to resize overlay:", error));
}


function handleLayoutChange(payload: {
  showSuggestions: boolean;
  suggestionCount: number;
  showModelMenu: boolean;
  modelMenuHeight: number;
  askUserRowCount?: number;
  pickerRowCount?: number;
  hasContextPreview?: boolean;
  mode?: "input" | "chat";
  hasImages?: boolean;
}) {
  const pickerHeight =
    (payload.pickerRowCount ?? 0) > 0
      ? computePickerHeight(payload.pickerRowCount ?? 0)
      : payload.showSuggestions
        ? SUGGESTION_PADDING + payload.suggestionCount * SUGGESTION_ROW_HEIGHT
        : 0;
  const modeValue = payload.mode ?? mode.value;
  // Floating model/approval menus are position:fixed — only the compact input
  // window needs extra height so the menu isn't clipped. Chat mode already has
  // room; resizing there just jumps the message panel.
  const modelMenuHeight =
    modeValue === "input" && payload.showModelMenu ? payload.modelMenuHeight : 0;
  const contextHeight = payload.hasContextPreview ? CONTEXT_PREVIEW_HEIGHT : 0;
  const imagesHeight = payload.hasImages ? 60 : 0;
  const extraHeight = pickerHeight + modelMenuHeight + contextHeight + imagesHeight;

  if (modeValue === "input") {
    chatWindowInitialized.value = false;
    lastComposerExtraHeight.value = 0;
    queueLayoutResize(() =>
      resizeWindow(PANEL_WIDTH, INPUT_BAR_HEIGHT + extraHeight, false, false, "bottom"),
    );
    return;
  }

  if (modeValue === "chat") {
    const deltaExtra = extraHeight - lastComposerExtraHeight.value;
    lastComposerExtraHeight.value = extraHeight;

    if (!chatWindowInitialized.value) {
      chatWindowInitialized.value = true;
      queueLayoutResize(() =>
        resizeWindow(PANEL_WIDTH, CHAT_HEIGHT + extraHeight, true),
      );
      return;
    }

    queueLayoutResize(() => adjustWindowHeightBy(deltaExtra, true));
  }
}

async function enterChatMode(nextSessionId: string) {
  sessionId.value = nextSessionId;
  chatStore.setOverlayDraftSession(nextSessionId);
  mode.value = "chat";
  chatWindowInitialized.value = false;
  lastComposerExtraHeight.value = 0;
  await setOverlayChatMode(windowLabel, true);
  // 先切换 UI、等一帧绘制，再以底边为锚点展开，让输入框保持原位
  await waitForNextFrame();
  await applySizeConstraints("chat", OVERLAY_MIN_HEIGHT_CHAT);
  await resizeWindow(PANEL_WIDTH, CHAT_HEIGHT, true, false, "bottom");
  chatWindowInitialized.value = true;
}

async function resetToInputMode() {
  mode.value = "input";
  sessionId.value = "";
  chatWindowInitialized.value = false;
  lastComposerExtraHeight.value = 0;
  chatStore.setOverlayDraftSession("");
  await setOverlayChatMode(windowLabel, false);
  await applySizeConstraints("input", OVERLAY_MIN_HEIGHT_INPUT);
  await resizeWindow(PANEL_WIDTH, INPUT_HEIGHT, false, false, "bottom");
}

async function close() {
  // 直接通知 Rust 关闭/销毁，由 Rust 侧负责清理状态
  // 不能先 resetToInputMode()，否则会提前清除 chat mode 导致竞态
  await closeOverlay(windowLabel);
}

onMounted(async () => {
  const window = getCurrentWebviewWindow();
  void window.setMaximizable(false);
  void window.listen<CapturedContext>(IPC_EVENTS.contextCaptured, (event) => {
    if (mode.value === "chat") {
      return;
    }
    capturedContext.value = event.payload;
  });
  const pendingContext = await takeOverlayContext(windowLabel);
  if (pendingContext && mode.value === "input") {
    capturedContext.value = pendingContext;
  }
  // 基础 overlay 窗口：监听 overlay-hidden 重置 UI
  // 动态窗口（overlay-N）即将被销毁，不需要 reset UI/resize
  const isBaseOverlay = windowLabel === "overlay";
  if (isBaseOverlay) {
    void applySizeConstraints("input", OVERLAY_MIN_HEIGHT_INPUT);
    void window.listen("overlay-hidden", () => {
      capturedContext.value = null;
      void resetToInputMode();
    });
  }
});

watch(
  () => settingStore.zoom,
  async () => {
    if (mode.value === "chat") {
      await applySizeConstraints("chat", OVERLAY_MIN_HEIGHT_CHAT);
      const window = getCurrentWebviewWindow();
      const scaleFactor = await window.scaleFactor();
      const logicalSize = (await window.outerSize()).toLogical(scaleFactor);
      const zoom = (settingStore.zoom || 100) / 100;
      await resizeWindow(logicalSize.width / zoom, logicalSize.height / zoom, true);
    } else {
      await applySizeConstraints("input", OVERLAY_MIN_HEIGHT_INPUT);
      await resizeWindow(PANEL_WIDTH, INPUT_HEIGHT, false, false, "bottom");
    }
  }
);
</script>

<style scoped>
.overlay-shell {
  box-sizing: border-box;
  width: 100%;
  height: 100%;
  overflow: hidden;
  background: transparent;
}
</style>

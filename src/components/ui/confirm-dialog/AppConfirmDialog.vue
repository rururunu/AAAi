<template>
  <DialogRoot :open="open" @update:open="handleOpenChange">
    <DialogPortal>
      <DialogOverlay class="confirm-overlay" />
      <DialogContent class="confirm-dialog" :aria-describedby="undefined">
        <div class="confirm-body">
          <span class="confirm-icon" aria-hidden="true">
            <TriangleAlert :size="16" />
          </span>
          <div class="confirm-copy">
            <DialogTitle class="confirm-title">{{ options.title }}</DialogTitle>
            <DialogDescription class="confirm-description">
              {{ options.description }}
            </DialogDescription>
          </div>
        </div>

        <div class="confirm-actions">
          <button type="button" class="confirm-button ghost" @click="settle(false)">
            {{ options.cancelLabel }}
          </button>
          <button type="button" class="confirm-button danger" @click="settle(true)">
            {{ options.confirmLabel }}
          </button>
        </div>
      </DialogContent>
    </DialogPortal>
  </DialogRoot>
</template>

<script setup lang="ts">
import { reactive, ref } from "vue";
import { TriangleAlert } from "@lucide/vue";
import {
  DialogContent,
  DialogDescription,
  DialogOverlay,
  DialogPortal,
  DialogRoot,
  DialogTitle,
} from "reka-ui";

export interface ConfirmDialogOptions {
  title: string;
  description: string;
  confirmLabel: string;
  cancelLabel: string;
}

const open = ref(false);
const options = reactive<ConfirmDialogOptions>({
  title: "",
  description: "",
  confirmLabel: "Confirm",
  cancelLabel: "Cancel",
});
let resolver: ((confirmed: boolean) => void) | null = null;

/** Open the dialog and resolve when the user confirms or cancels. */
function ask(nextOptions: ConfirmDialogOptions) {
  resolver?.(false);
  Object.assign(options, nextOptions);
  open.value = true;
  return new Promise<boolean>((resolve) => {
    resolver = resolve;
  });
}

/** Close the dialog and settle the pending promise. */
function settle(confirmed: boolean) {
  open.value = false;
  resolver?.(confirmed);
  resolver = null;
}

/** Treat outside-dismiss / Escape as cancel. */
function handleOpenChange(nextOpen: boolean) {
  if (!nextOpen && open.value) settle(false);
}

defineExpose({ ask });
</script>

<style>
.confirm-overlay {
  position: fixed;
  inset: 0;
  z-index: 50;
  background: color-mix(in srgb, #000 48%, transparent);
  backdrop-filter: blur(2px);
}

.confirm-dialog {
  position: fixed;
  top: 50%;
  left: 50%;
  z-index: 51;
  box-sizing: border-box;
  width: min(360px, calc(100vw - 32px));
  padding: 16px;
  border: 1px solid var(--peek-border, rgba(255, 255, 255, 0.14));
  border-radius: 12px;
  background: var(--peek-dialog-bg, var(--peek-surface, #252526));
  color: var(--peek-text, #f3f4f6);
  box-shadow: 0 18px 48px var(--peek-shadow, rgb(0 0 0 / 28%));
  transform: translate(-50%, -50%);
  outline: none;
}

.confirm-dialog .confirm-body {
  display: flex;
  align-items: flex-start;
  gap: 12px;
}

.confirm-dialog .confirm-icon {
  flex: none;
  display: grid;
  place-items: center;
  width: 32px;
  height: 32px;
  margin-top: 1px;
  border-radius: 8px;
  background: color-mix(in srgb, var(--peek-danger, #f14c4c) 14%, transparent);
  color: var(--peek-danger, #f14c4c);
}

.confirm-dialog .confirm-copy {
  min-width: 0;
  flex: 1;
}

.confirm-dialog .confirm-title {
  margin: 0;
  color: var(--peek-text, #f3f4f6);
  font-size: 14px;
  font-weight: 650;
  line-height: 1.35;
}

.confirm-dialog .confirm-description {
  margin: 6px 0 0;
  color: var(--peek-muted, #b7bcc5);
  font-size: 12px;
  line-height: 1.55;
  white-space: pre-line;
}

.confirm-dialog .confirm-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 16px;
}

.confirm-dialog .confirm-button {
  min-width: 72px;
  height: 30px;
  padding: 0 12px;
  border: 1px solid transparent;
  border-radius: 7px;
  font: inherit;
  font-size: 12px;
  font-weight: 550;
  cursor: pointer;
}

.confirm-dialog .confirm-button.ghost {
  border-color: var(--peek-border, rgba(255, 255, 255, 0.14));
  background: color-mix(in srgb, var(--peek-text, #f3f4f6) 4%, transparent);
  color: var(--peek-text, #f3f4f6);
}

.confirm-dialog .confirm-button.ghost:hover {
  background: var(--peek-hover-bg, color-mix(in srgb, var(--peek-icon, #e5e7eb) 9%, transparent));
}

.confirm-dialog .confirm-button.danger {
  border-color: color-mix(in srgb, var(--peek-danger, #f14c4c) 35%, transparent);
  background: color-mix(in srgb, var(--peek-danger, #f14c4c) 18%, transparent);
  color: color-mix(in srgb, var(--peek-danger, #f14c4c) 88%, var(--peek-text, #f3f4f6));
}

.confirm-dialog .confirm-button.danger:hover {
  background: color-mix(in srgb, var(--peek-danger, #f14c4c) 28%, transparent);
}

[data-theme="light"] .confirm-dialog {
  background: var(--peek-surface, #ffffff);
  color: var(--peek-text, #242424);
  box-shadow: 0 18px 48px var(--peek-shadow, rgb(0 0 0 / 18%));
}
</style>

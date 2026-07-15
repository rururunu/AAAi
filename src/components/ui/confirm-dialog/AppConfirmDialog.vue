<template>
  <DialogRoot :open="open" @update:open="handleOpenChange">
    <DialogPortal>
      <DialogOverlay class="fixed inset-0 z-50 bg-black/20 backdrop-blur-[2px] data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0" />
      <DialogContent
        class="fixed top-1/2 left-1/2 z-50 w-[min(360px,calc(100vw-32px))] -translate-x-1/2 -translate-y-1/2 rounded-lg border border-border bg-background p-4 text-foreground shadow-xl outline-none data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95"
      >
        <div class="flex items-start gap-3">
          <span class="mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-md bg-destructive/10 text-destructive">
            <TriangleAlert class="size-4" />
          </span>
          <div class="min-w-0 flex-1">
            <DialogTitle class="text-sm font-semibold">{{ options.title }}</DialogTitle>
            <DialogDescription class="mt-1.5 whitespace-pre-line text-xs leading-5 text-muted-foreground">
              {{ options.description }}
            </DialogDescription>
          </div>
        </div>

        <div class="mt-4 flex justify-end gap-2">
          <Button variant="outline" size="sm" @click="settle(false)">
            {{ options.cancelLabel }}
          </Button>
          <Button variant="destructive" size="sm" @click="settle(true)">
            {{ options.confirmLabel }}
          </Button>
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
import { Button } from "@/components/ui/button";

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

function ask(nextOptions: ConfirmDialogOptions) {
  resolver?.(false);
  Object.assign(options, nextOptions);
  open.value = true;
  return new Promise<boolean>((resolve) => {
    resolver = resolve;
  });
}

function settle(confirmed: boolean) {
  open.value = false;
  resolver?.(confirmed);
  resolver = null;
}

function handleOpenChange(nextOpen: boolean) {
  if (!nextOpen && open.value) settle(false);
}

defineExpose({ ask });
</script>

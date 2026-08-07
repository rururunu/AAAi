<template>
  <AppErrorBoundary>
    <RouterView v-slot="{ Component }">
      <Suspense>
        <component :is="Component" class="h-full w-full" />
        <template #fallback>
          <!-- Solid fill only — boot splash stays on top until Main is painted.
               Avoid a second animated loader that vanishes when the route resolves. -->
          <div v-if="isWorkbench" class="route-boot-fill" aria-hidden="true" />
          <div v-else class="route-loading" />
        </template>
      </Suspense>
    </RouterView>
  </AppErrorBoundary>
  <AppTooltipLayer />
</template>

<script setup lang="ts">
import { RouterView } from "vue-router";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import AppErrorBoundary from "@/components/AppErrorBoundary.vue";
import AppTooltipLayer from "@/components/ui/AppTooltipLayer.vue";

const isWorkbench = getCurrentWebviewWindow().label === "workbench";
</script>

<style scoped>
.route-loading,
.route-boot-fill {
  width: 100%;
  height: 100%;
  background: var(--peek-bg, #1f1f1f);
}
</style>

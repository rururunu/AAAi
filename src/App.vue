<template>
  <RouterView v-slot="{ Component }">
    <Suspense>
      <component :is="Component" class="h-full w-full" />
      <template #fallback>
        <WorkbenchLoading v-if="isWorkbench" />
        <div v-else class="route-loading" />
      </template>
    </Suspense>
  </RouterView>
  <AppTooltipLayer />
</template>

<script setup lang="ts">
import { RouterView } from "vue-router";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import WorkbenchLoading from "@/components/workbench/WorkbenchLoading.vue";
import AppTooltipLayer from "@/components/ui/AppTooltipLayer.vue";

const isWorkbench = getCurrentWebviewWindow().label === "workbench";
</script>

<style scoped>
.route-loading { width: 100%; height: 100%; }
</style>

<template>
  <div class="workbench-loading" role="status" :aria-label="label">
    <div class="loading-brand" aria-hidden="true">
      <img class="loading-logo" :src="appIconAsset" alt="" draggable="false" />
    </div>
    <p class="loading-label">{{ label }}</p>
    <span class="loading-progress" aria-hidden="true"><i /></span>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useSettingStore } from "@/stores/setting";
import appIconAsset from "../../../src-tauri/icons/AAAi-transparent.svg";

const settingStore = useSettingStore();

const label = computed(() =>
  settingStore.language === "zh-CN" || navigator.language.toLowerCase().startsWith("zh")
    ? "请稍等，正在为您准备……"
    : "Please wait, preparing for you…",
);
</script>

<style scoped>
.workbench-loading {
  position: fixed;
  z-index: 1000;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 20px;
  background:
    radial-gradient(120% 80% at 50% -10%, #2a2a2a 0%, transparent 55%),
    linear-gradient(180deg, #1f1f1f 0%, #181818 100%);
  color: #f3f4f6;
  font-family: var(--peek-font-sans, "Noto Sans SC", "Segoe UI", sans-serif);
  user-select: none;
}

.workbench[data-theme="light"] .workbench-loading,
:global(html[data-theme="light"]) .workbench-loading {
  background:
    radial-gradient(120% 80% at 50% -10%, #ffffff 0%, transparent 55%),
    linear-gradient(180deg, #f7f5f1 0%, #efeae2 52%, #e8e2d8 100%);
  color: #1c1915;
}

.loading-brand {
  position: relative;
  width: 168px;
  height: 168px;
  display: grid;
  place-items: center;
}

.loading-logo {
  width: 168px;
  height: 168px;
  object-fit: contain;
  filter: invert(1);
  animation: logo-breathe 2.2s ease-in-out infinite;
}

.workbench[data-theme="light"] .loading-logo,
:global(html[data-theme="light"]) .loading-logo {
  filter: none;
}

.loading-label {
  margin: 0;
  max-width: min(80vw, 360px);
  color: rgba(243, 244, 246, 0.62);
  font-size: 14px;
  font-weight: 500;
  letter-spacing: 0.02em;
  text-align: center;
}

.workbench[data-theme="light"] .loading-label,
:global(html[data-theme="light"]) .loading-label {
  color: rgba(28, 25, 21, 0.62);
}

.loading-progress {
  width: 120px;
  height: 2px;
  overflow: hidden;
  border-radius: 999px;
  background: rgba(243, 244, 246, 0.12);
}

.workbench[data-theme="light"] .loading-progress,
:global(html[data-theme="light"]) .loading-progress {
  background: rgba(28, 25, 21, 0.08);
}

.loading-progress i {
  display: block;
  width: 36%;
  height: 100%;
  border-radius: inherit;
  background: #f3f4f6;
  animation: progress-travel 1.45s ease-in-out infinite;
}

.workbench[data-theme="light"] .loading-progress i,
:global(html[data-theme="light"]) .loading-progress i {
  background: #171411;
}

@keyframes logo-breathe {
  0%,
  100% {
    transform: scale(1);
    opacity: 0.92;
  }
  50% {
    transform: scale(1.04);
    opacity: 1;
  }
}

@keyframes progress-travel {
  0% {
    transform: translateX(-120%);
    opacity: 0.45;
  }
  45% {
    opacity: 1;
  }
  100% {
    transform: translateX(320%);
    opacity: 0.45;
  }
}

@media (prefers-reduced-motion: reduce) {
  .loading-logo,
  .loading-progress i {
    animation: none;
  }
}
</style>

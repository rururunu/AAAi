<template>
  <div class="workbench-loading" role="status" :aria-label="label">
    <div class="loading-brand" aria-hidden="true">
      <img class="loading-logo" :src="appIconAsset" alt="" draggable="false" />
      <span class="loading-halo" />
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
  gap: 18px;
  background:
    radial-gradient(120% 80% at 50% -10%, #ffffff 0%, transparent 55%),
    linear-gradient(180deg, #f7f5f1 0%, #efeae2 52%, #e8e2d8 100%);
  color: #1c1915;
  font-family: var(--peek-font-sans, "Noto Sans SC", "Segoe UI", sans-serif);
  user-select: none;
}

.workbench[data-theme="dark"] .workbench-loading,
:global(html.dark) .workbench-loading {
  background:
    radial-gradient(120% 80% at 50% -10%, #2a2a2a 0%, transparent 55%),
    linear-gradient(180deg, #1f1f1f 0%, #181818 100%);
  color: #f3f4f6;
}

.loading-brand {
  position: relative;
  width: 112px;
  height: 112px;
  display: grid;
  place-items: center;
}

.loading-logo {
  position: relative;
  z-index: 1;
  width: 88px;
  height: 88px;
  object-fit: contain;
  animation: logo-breathe 2.2s ease-in-out infinite;
}

:global(html.dark) .loading-logo,
.workbench[data-theme="dark"] .loading-logo {
  filter: invert(1);
}

.loading-halo {
  position: absolute;
  inset: 0;
  border-radius: 50%;
  border: 1px solid rgba(28, 25, 21, 0.1);
  animation: halo-pulse 2.2s ease-in-out infinite;
}

:global(html.dark) .loading-halo,
.workbench[data-theme="dark"] .loading-halo {
  border-color: rgba(255, 255, 255, 0.12);
}

.loading-label {
  margin: 0;
  max-width: min(80vw, 360px);
  color: rgba(28, 25, 21, 0.62);
  font-size: 14px;
  font-weight: 500;
  letter-spacing: 0.02em;
  text-align: center;
}

:global(html.dark) .loading-label,
.workbench[data-theme="dark"] .loading-label {
  color: rgba(243, 244, 246, 0.62);
}

.loading-progress {
  width: 120px;
  height: 2px;
  overflow: hidden;
  border-radius: 999px;
  background: rgba(28, 25, 21, 0.08);
}

:global(html.dark) .loading-progress,
.workbench[data-theme="dark"] .loading-progress {
  background: rgba(255, 255, 255, 0.1);
}

.loading-progress i {
  display: block;
  width: 36%;
  height: 100%;
  border-radius: inherit;
  background: #171411;
  animation: progress-travel 1.45s ease-in-out infinite;
}

:global(html.dark) .loading-progress i,
.workbench[data-theme="dark"] .loading-progress i {
  background: #f3f4f6;
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

@keyframes halo-pulse {
  0%,
  100% {
    transform: scale(0.92);
    opacity: 0.55;
  }
  50% {
    transform: scale(1.08);
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
  .loading-halo,
  .loading-progress i {
    animation: none;
  }
}
</style>

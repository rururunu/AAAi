<template>
  <div class="workbench-loading" role="status" :aria-label="label">
    <div class="loading-mark" aria-hidden="true">
      <span class="mark-letter">A</span>
      <span class="orbit orbit-one" />
      <span class="orbit orbit-two" />
    </div>
    <strong>AAAi</strong>
    <span class="loading-label">{{ label }}</span>
    <span class="loading-progress" aria-hidden="true"><i /></span>
  </div>
</template>

<script setup lang="ts">
const language = navigator.language.toLowerCase();
const label = language.startsWith("zh") ? "\u6b63\u5728\u51c6\u5907\u5de5\u4f5c\u53f0" : "Preparing workbench";
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
  background: var(--peek-bg, #1f1f1f);
  color: var(--peek-text, #d4d4d4);
  font-family: var(--peek-font-sans, "Segoe UI", sans-serif);
  user-select: none;
}

.loading-mark {
  position: relative;
  width: 52px;
  height: 52px;
  display: grid;
  place-items: center;
  margin-bottom: 16px;
}
.mark-letter {
  position: relative;
  z-index: 2;
  font-size: 17px;
  font-weight: 680;
  line-height: 1;
  color: var(--peek-text, #d4d4d4);
}
.orbit {
  position: absolute;
  inset: 5px;
  border: 1px solid color-mix(in srgb, var(--peek-text, #d4d4d4) 14%, transparent);
  border-radius: 50%;
}
.orbit-one {
  border-top-color: var(--peek-accent, #3b8eea);
  animation: orbit-clockwise 1.4s linear infinite;
}
.orbit-two {
  inset: 11px;
  border-right-color: color-mix(in srgb, var(--peek-accent, #3b8eea) 65%, var(--peek-text, #d4d4d4));
  animation: orbit-counter 1.9s linear infinite;
}
.workbench-loading > strong { font-size: 14px; font-weight: 650; }
.loading-label { margin-top: 7px; color: var(--peek-muted, #a0a0a0); font-size: 11px; }
.loading-progress {
  width: 86px;
  height: 2px;
  margin-top: 17px;
  overflow: hidden;
  border-radius: 1px;
  background: color-mix(in srgb, var(--peek-text, #d4d4d4) 9%, transparent);
}
.loading-progress i {
  display: block;
  width: 30%;
  height: 100%;
  border-radius: inherit;
  background: var(--peek-accent, #3b8eea);
  animation: progress-travel 1.45s ease-in-out infinite;
}

@keyframes orbit-clockwise { to { transform: rotate(360deg); } }
@keyframes orbit-counter { to { transform: rotate(-360deg); } }
@keyframes progress-travel {
  0% { transform: translateX(-110%); opacity: 0.45; }
  45% { opacity: 1; }
  100% { transform: translateX(340%); opacity: 0.45; }
}

@media (prefers-reduced-motion: reduce) {
  .orbit-one, .orbit-two, .loading-progress i { animation-duration: 3.5s; }
}
</style>

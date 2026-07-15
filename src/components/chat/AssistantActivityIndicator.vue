<template>
  <div class="assistant-activity" role="status" aria-live="polite">
    <LoaderCircle class="activity-spinner" :size="14" aria-hidden="true" />
    <span class="activity-label">{{ label }}</span>
    <span class="activity-dots" aria-hidden="true">
      <span></span>
      <span></span>
      <span></span>
    </span>
  </div>
</template>

<script setup lang="ts">
import { LoaderCircle } from "@lucide/vue";

defineProps<{
  label: string;
}>();
</script>

<style scoped>
.assistant-activity {
  display: inline-flex;
  align-items: center;
  align-self: flex-start;
  gap: 6px;
  min-height: 22px;
  color: var(--peek-muted);
  font-size: 12px;
  line-height: 1;
}

.activity-spinner {
  flex: none;
  color: var(--peek-accent);
  animation: activity-spin 1.1s linear infinite;
}

.activity-label {
  min-width: 3em;
}

.activity-dots {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  width: 24px;
  height: 10px;
}

.activity-dots span {
  width: 4px;
  height: 4px;
  border-radius: 50%;
  background: currentColor;
  animation: activity-bounce 1.2s ease-in-out infinite;
}

.activity-dots span:nth-child(2) { animation-delay: 140ms; }
.activity-dots span:nth-child(3) { animation-delay: 280ms; }

@keyframes activity-spin {
  to { transform: rotate(360deg); }
}

@keyframes activity-bounce {
  0%, 60%, 100% { opacity: 0.3; transform: translateY(0); }
  30% { opacity: 1; transform: translateY(-3px); }
}

@media (prefers-reduced-motion: reduce) {
  .activity-spinner { animation-duration: 2.4s; }
  .activity-dots span { animation: none; opacity: 0.65; }
}
</style>

<script setup lang="ts">
import { onErrorCaptured, ref } from "vue";
import { createLogger } from "@/services/logger";

const log = createLogger("error-boundary");
const isDev = import.meta.env.DEV;

const props = withDefaults(
  defineProps<{
    /** Compact overlay-friendly fallback. */
    compact?: boolean;
  }>(),
  { compact: false },
);

const errorMessage = ref<string | null>(null);
const errorStack = ref<string | null>(null);

onErrorCaptured((err, _instance, info) => {
  const message = err instanceof Error ? err.message : String(err);
  const stack = err instanceof Error ? (err.stack ?? null) : null;
  errorMessage.value = message;
  errorStack.value = stack;
  log.error("captured", { message, info, stack });
  return false;
});

function retry() {
  errorMessage.value = null;
  errorStack.value = null;
}
</script>

<template>
  <div v-if="errorMessage" class="app-error-boundary" :class="{ compact: props.compact }">
    <p class="title">Something went wrong</p>
    <p class="message">{{ errorMessage }}</p>
    <pre v-if="isDev && errorStack" class="stack">{{ errorStack }}</pre>
    <button type="button" class="retry" @click="retry">Retry</button>
  </div>
  <slot v-else />
</template>

<style scoped>
.app-error-boundary {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  align-items: flex-start;
  justify-content: center;
  width: 100%;
  height: 100%;
  padding: 1.5rem;
  color: #e8e8e8;
  background: #1f1f1f;
  box-sizing: border-box;
}

.app-error-boundary.compact {
  padding: 1rem;
  min-height: 8rem;
}

.title {
  margin: 0;
  font-size: 1rem;
  font-weight: 600;
}

.message {
  margin: 0;
  font-size: 0.875rem;
  opacity: 0.85;
  word-break: break-word;
}

.stack {
  margin: 0;
  max-width: 100%;
  max-height: 12rem;
  overflow: auto;
  padding: 0.75rem;
  font-size: 0.7rem;
  line-height: 1.4;
  white-space: pre-wrap;
  background: #151515;
  border-radius: 0.375rem;
}

.retry {
  appearance: none;
  border: 1px solid #4a4a4a;
  background: #2a2a2a;
  color: inherit;
  border-radius: 0.375rem;
  padding: 0.4rem 0.85rem;
  font-size: 0.8125rem;
  cursor: pointer;
}

.retry:hover {
  background: #333;
}
</style>

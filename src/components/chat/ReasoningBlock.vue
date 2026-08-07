<template>
  <details class="reasoning-block" :class="{ embedded }" :open="isOpen" @toggle="handleToggle">
    <summary class="reasoning-summary">
      <ChevronRight class="reasoning-chevron" :class="{ open: isOpen }" :size="12" />
      <span>{{ summaryLabel }}</span>
      <span v-if="!isOpen" class="reasoning-meta">{{ collapsedHint }}</span>
    </summary>
    <div v-if="isOpen" ref="bodyRef" class="reasoning-body peek-scrollbar">{{ displayText }}</div>
  </details>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { ChevronRight } from "@lucide/vue";
import { displayReasoningText } from "@/services/chat/reasoningDisplay";
import type { AppLanguage } from "@/types/setting";
import { tr } from "@/services/i18n";

const props = defineProps<{
  reasoning: string;
  streaming?: boolean;
  language?: AppLanguage;
  /** Nested under the agent work stream: lighter chrome, same collapse rules. */
  embedded?: boolean;
}>();

const isOpen = ref(false);
/** After the turn finishes, honor manual expand/collapse until streaming resumes. */
const userPinned = ref(false);

const summaryLabel = computed(() => tr(props.language, "thinkingProcess"));

const collapsedHint = computed(() => {
  const chars = props.reasoning.length;
  return tr(props.language, "chars", { count: chars.toLocaleString() });
});

const displayText = computed(() =>
  displayReasoningText(props.reasoning, {
    streaming: props.streaming ?? false,
  }),
);

watch(
  () => props.streaming,
  (streaming) => {
    if (streaming) {
      userPinned.value = false;
      isOpen.value = true;
      return;
    }
    // Always collapse when the segment is no longer actively streaming.
    // (Previously only collapsed on a true→false transition, so remounts /
    // missed transitions could leave the full thinking body open after done.)
    if (!userPinned.value) {
      isOpen.value = false;
    }
  },
  { immediate: true },
);

function handleToggle(event: Event) {
  const target = event.currentTarget as HTMLDetailsElement | null;
  if (!target) {
    return;
  }
  isOpen.value = target.open;
  if (!props.streaming) {
    userPinned.value = target.open;
  }
}

const bodyRef = ref<HTMLElement | null>(null);

watch(
  () => props.reasoning,
  () => {
    if (props.streaming) {
      nextTick(() => {
        const el = bodyRef.value;
        if (el) {
          el.scrollTop = el.scrollHeight;
        }
      });
    }
  },
);
</script>

<style scoped>
.reasoning-block {
  width: 100%;
  margin-bottom: 0;
  border: 1px solid color-mix(in srgb, var(--peek-border) 88%, var(--peek-muted));
  border-radius: 8px;
  background: var(--peek-surface);
  isolation: isolate;
  box-sizing: border-box;
}

.reasoning-summary {
  display: flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  padding: 6px 10px;
  font-family: var(--peek-font-sans);
  font-size: 12px;
  font-weight: 600;
  color: var(--peek-muted);
  list-style: none;
  user-select: none;
}

.reasoning-summary::-webkit-details-marker {
  display: none;
}

.reasoning-chevron {
  flex: none;
  color: var(--peek-faint);
  transition: transform 160ms ease;
}

.reasoning-chevron.open {
  transform: rotate(90deg);
}

.reasoning-meta {
  margin-left: auto;
  font-weight: 500;
  font-size: 11px;
  color: var(--peek-faint);
}

.reasoning-body {
  margin: 0;
  padding: 4px 12px 10px;
  max-height: min(40vh, 280px);
  overflow: auto;
  white-space: pre-wrap;
  word-break: break-word;
  font-family: var(--peek-font-sans);
  font-size: 13px;
  font-weight: 400;
  line-height: 1.65;
  letter-spacing: 0.01em;
  color: var(--peek-text);
  -webkit-font-smoothing: subpixel-antialiased;
  transform: translateZ(0);
}

/* Nested under process panel: no second card chrome */
.reasoning-block.embedded {
  margin-bottom: 0;
  border: 0;
  border-radius: 0;
  background: transparent;
  isolation: auto;
}

.reasoning-block.embedded .reasoning-summary {
  padding: 3px 2px;
  font-size: 11px;
  font-weight: 550;
}

.reasoning-block.embedded .reasoning-body {
  padding: 2px 2px 6px;
  max-height: min(32vh, 220px);
  font-size: 12px;
  line-height: 1.55;
  color: var(--peek-muted);
}
</style>

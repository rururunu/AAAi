<template>
  <ul
    class="command-list history-list peek-scrollbar"
    data-tauri-drag-region="false"
    role="listbox"
    :aria-label="ariaLabel"
  >
    <li
      v-for="(item, index) in items"
      :key="item.sessionId"
      class="command-item history-item"
      :class="{ active: index === selectedIndex }"
      role="option"
      :aria-selected="index === selectedIndex"
      @mouseenter="$emit('hover', index)"
      @mousedown.prevent="$emit('select', item.sessionId)"
    >
      <span class="history-preview">{{ item.preview }}</span>
      <span v-if="item.estimatedTokens" class="history-tokens">
        ≈{{ formatTokenCount(item.estimatedTokens) }} tokens
      </span>
      <span class="history-time">{{ formatTime(item.updatedAt) }}</span>
    </li>
  </ul>
</template>

<script setup lang="ts">
import type { ChatSessionSummary } from "@/types/chat";
import { formatTokenCount } from "@/services/chat/tokenEstimate";

defineProps<{
  items: ChatSessionSummary[];
  selectedIndex: number;
  ariaLabel: string;
  formatTime: (timestamp: number) => string;
}>();

defineEmits<{
  hover: [index: number];
  select: [sessionId: string];
}>();
</script>

<style scoped>
.command-list {
  --command-row-height: 32px;
  --command-list-padding: 6px;
  --command-list-visible-rows: 8;
  list-style: none;
  margin: 0;
  padding: 4px 0;
  border-bottom: 1px solid var(--peek-border);
  background: var(--peek-list-bg);
  flex: none;
  max-height: min(
    calc(
      var(--command-row-height) * var(--command-list-visible-rows) +
        var(--command-list-padding)
    ),
    72vh
  );
  overflow-x: hidden;
  overflow-y: auto;
  overscroll-behavior: contain;
}

.command-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0 12px;
  height: var(--command-row-height);
  cursor: default;
}

.command-item.active {
  background: var(--peek-list-active);
}

.history-preview {
  flex: 1;
  min-width: 0;
  font-size: 13px;
  font-weight: 500;
  color: var(--peek-text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.history-time, .history-tokens {
  flex: none;
  font-size: 11px;
  color: var(--peek-muted);
}
.history-tokens { font-size: 10px; font-variant-numeric: tabular-nums; }
</style>

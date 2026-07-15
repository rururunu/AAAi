<template>
  <ul
    class="command-list ask-user-list history-list peek-scrollbar"
    data-tauri-drag-region="false"
    role="listbox"
    :aria-label="ariaLabel"
  >
    <li class="picker-meta">
      <span class="picker-meta-label">{{ header }}</span>
    </li>
    <li v-if="items.length === 0" class="picker-meta question">
      {{ emptyText }}
    </li>
    <li
      v-for="(item, index) in items"
      :key="item.sessionId"
      class="command-item"
      :class="{ active: index === selectedIndex }"
      role="option"
      :aria-selected="index === selectedIndex"
      @mouseenter="$emit('hover', index)"
      @mousedown.prevent="$emit('select', item.sessionId)"
    >
      <span class="command-desc" style="color: var(--peek-text); font-size: 13px; font-weight: 500;">{{ item.preview }}</span>
      <span class="command-time" style="font-size: 11px; color: var(--peek-muted); margin-left: 8px; flex: none;">{{ formatTime(item.updatedAt) }}</span>
    </li>
  </ul>
</template>

<script setup lang="ts">
import type { ChatSessionSummary } from "@/types/chat";

defineProps<{
  items: ChatSessionSummary[];
  selectedIndex: number;
  header: string;
  emptyText: string;
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
  --command-row-height: 30px;
  --command-list-padding: 8px;
  --picker-meta-row-height: 28px;
  --command-list-visible-rows: 8;
  list-style: none;
  margin: 0;
  padding: 4px 0;
  border-bottom: 1px solid var(--peek-border);
  background: var(--peek-list-bg);
  flex: none;
  max-height: min(
    calc(var(--command-row-height) * var(--command-list-visible-rows) + var(--command-list-padding)),
    72vh
  );
  overflow-x: hidden;
  overflow-y: auto;
  overscroll-behavior: contain;
}

.ask-user-list {
  max-height: min(
    calc(
      var(--picker-meta-row-height) * 2 +
        var(--command-row-height) * var(--command-list-visible-rows) +
        var(--command-list-padding)
    ),
    72vh
  );
}

.history-list {
  max-height: min(
    calc(
      var(--picker-meta-row-height) * 2 +
        var(--command-row-height) * var(--command-list-visible-rows) +
        var(--command-list-padding)
    ),
    72vh
  );
}

.ask-user-list .picker-meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 0 12px;
  min-height: 26px;
  font-size: 11px;
  color: var(--peek-muted);
  pointer-events: none;
}

.ask-user-list .picker-meta.question {
  min-height: 28px;
  align-items: flex-start;
  padding-top: 2px;
  padding-bottom: 4px;
  line-height: 1.45;
  color: var(--peek-text);
  white-space: normal;
}

.picker-meta-label {
  font-weight: 600;
  color: var(--peek-accent);
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

.command-desc {
  flex: 1;
  min-width: 0;
  font-size: 12px;
  color: var(--peek-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>

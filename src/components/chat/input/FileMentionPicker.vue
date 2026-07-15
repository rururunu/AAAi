<template>
  <ul
    class="command-list file-suggestion-list peek-scrollbar"
    data-tauri-drag-region="false"
    role="listbox"
    :aria-label="ariaLabel"
  >
    <li v-if="loading" class="picker-meta question">{{ loadingText }}</li>
    <li v-else-if="suggestions.length === 0" class="picker-meta question">{{ emptyText }}</li>
    <li
      v-for="(path, index) in suggestions"
      v-else
      :key="path"
      class="command-item file-suggestion-item"
      :class="{ active: index === selectedIndex }"
      role="option"
      :aria-selected="index === selectedIndex"
      @mouseenter="$emit('hover', index)"
      @mousedown.prevent="$emit('select', path)"
    >
      <File :size="13" class="file-suggestion-icon" />
      <span class="command-desc">{{ path }}</span>
    </li>
  </ul>
</template>

<script setup lang="ts">
import { File } from "@lucide/vue";

defineProps<{
  loading: boolean;
  suggestions: string[];
  selectedIndex: number;
  loadingText: string;
  emptyText: string;
  ariaLabel: string;
}>();

defineEmits<{
  hover: [index: number];
  select: [path: string];
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

.file-suggestion-item {
  gap: 8px;
}

.file-suggestion-list {
  --command-list-visible-rows: 12;
}

.file-suggestion-icon {
  flex: none;
  color: var(--peek-muted);
}
</style>

<template>
  <ul
    class="command-list hash-suggestion-list peek-scrollbar"
    data-tauri-drag-region="false"
    role="listbox"
    :aria-label="ariaLabel"
  >
    <li v-if="loading" class="picker-meta question">{{ loadingText }}</li>
    <li v-else-if="items.length === 0" class="picker-meta question">{{ emptyText }}</li>
    <li
      v-for="(item, index) in items"
      v-else
      :key="`${item.kind}:${item.id}`"
      class="command-item hash-suggestion-item"
      :class="{ active: index === selectedIndex }"
      role="option"
      :aria-selected="index === selectedIndex"
      @mouseenter="$emit('hover', index)"
      @mousedown.prevent="$emit('select', item)"
    >
      <span class="hash-kind-badge" :data-kind="item.kind">{{ kindLabel(item.kind) }}</span>
      <div class="hash-suggestion-text">
        <span class="hash-suggestion-title">{{ item.title || item.id }}</span>
        <span v-if="item.description" class="command-desc">{{ item.description }}</span>
        <span v-else class="command-desc">#{{ item.kind }}:{{ item.id }}</span>
      </div>
    </li>
  </ul>
</template>

<script setup lang="ts">
import type { HashMentionItem, HashResourceKind } from "@/services/chat/hashMentions";

const props = defineProps<{
  loading: boolean;
  items: HashMentionItem[];
  selectedIndex: number;
  loadingText: string;
  emptyText: string;
  ariaLabel: string;
  skillLabel: string;
  mcpLabel: string;
}>();

defineEmits<{
  hover: [index: number];
  select: [item: HashMentionItem];
}>();

function kindLabel(kind: HashResourceKind): string {
  return kind === "skill" ? props.skillLabel : props.mcpLabel;
}
</script>

<style scoped>
.command-list {
  --command-row-height: 36px;
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
    calc(
      var(--command-row-height) * var(--command-list-visible-rows) + var(--command-list-padding)
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
  min-height: var(--command-row-height);
  cursor: default;
}

.command-item.active {
  background: color-mix(in srgb, var(--peek-accent) 14%, var(--peek-list-bg));
  color: var(--peek-accent);
}

.hash-kind-badge {
  flex: none;
  font-size: 10px;
  font-weight: 600;
  letter-spacing: 0.02em;
  padding: 2px 6px;
  border-radius: 4px;
  background: color-mix(in srgb, var(--peek-muted) 18%, transparent);
  color: var(--peek-muted);
  text-transform: uppercase;
}

.hash-kind-badge[data-kind="skill"] {
  background: color-mix(in srgb, var(--peek-accent) 16%, transparent);
  color: var(--peek-accent);
}

.hash-kind-badge[data-kind="mcp"] {
  background: color-mix(in srgb, #3b82f6 18%, transparent);
  color: #60a5fa;
}

.hash-suggestion-text {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.hash-suggestion-title {
  font-size: 12px;
  color: var(--peek-fg, inherit);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.command-desc {
  font-size: 11px;
  color: var(--peek-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.picker-meta {
  padding: 8px 12px;
  font-size: 12px;
  color: var(--peek-muted);
}
</style>

<template>
  <ul
    class="command-list ask-user-list path-permission-list tool-approval-list peek-scrollbar"
    data-tauri-drag-region="false"
    role="listbox"
    :aria-label="ariaLabel"
  >
    <li class="picker-meta">
      <span class="picker-meta-label">{{ header }}</span>
    </li>
    <li
      v-for="(option, index) in options"
      :key="option.slug"
      class="command-item"
      :class="{ active: index === selectedIndex }"
      role="option"
      :aria-selected="index === selectedIndex"
      @mouseenter="$emit('hover', index)"
      @mousedown.prevent="$emit('select', option.decision)"
    >
      <span class="permission-option-label">{{ option.label }}</span>
    </li>
  </ul>
</template>

<script setup lang="ts">
import type { ToolApprovalDecision } from "@/types/chat";

defineProps<{
  header: string;
  options: Array<{ slug: string; label: string; description: string; decision: ToolApprovalDecision }>;
  selectedIndex: number;
  ariaLabel: string;
}>();

defineEmits<{
  hover: [index: number];
  select: [decision: ToolApprovalDecision];
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

.path-permission-list {
  max-height: min(
    calc(
      var(--picker-meta-row-height) * 3 +
        var(--command-row-height) * var(--command-list-visible-rows) +
        var(--command-list-padding)
    ),
    72vh
  );
  border: 0;
  background: var(--peek-list-bg);
  -webkit-font-smoothing: antialiased;
  text-rendering: geometricPrecision;
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

.tool-approval-list {
  max-height: min(
    calc(
      var(--picker-meta-row-height) * 1 +
        var(--command-row-height) * var(--command-list-visible-rows) +
        var(--command-list-padding)
    ),
    72vh
  );
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

.path-permission-list .command-item {
  min-height: 32px;
  height: 32px;
  padding: 0 12px;
}

.permission-option-label {
  color: var(--peek-text);
  font-size: 12px;
  font-weight: 500;
}

.path-permission-list .command-item.active {
  background: color-mix(in srgb, var(--peek-text) 7%, transparent);
}
</style>

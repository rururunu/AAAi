<template>
  <ul
    class="command-list ask-user-list path-permission-list peek-scrollbar"
    data-tauri-drag-region="false"
    role="listbox"
    :aria-label="ariaLabel"
  >
    <li class="picker-meta">
      <span class="picker-meta-label">{{ header }}</span>
    </li>
    <li class="picker-meta question">{{ question }}</li>
    <li class="picker-meta path">{{ path }}</li>
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
import type { PathPermissionDecision } from "@/types/chat";

defineProps<{
  header: string;
  question: string;
  path?: string;
  options: Array<{ slug: string; label: string; description: string; decision: PathPermissionDecision }>;
  selectedIndex: number;
  ariaLabel: string;
}>();

defineEmits<{
  hover: [index: number];
  select: [decision: PathPermissionDecision];
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

.ask-user-list .picker-meta.question {
  min-height: 28px;
  align-items: flex-start;
  padding-top: 2px;
  padding-bottom: 4px;
  line-height: 1.45;
  color: var(--peek-text);
  white-space: normal;
}

.path-permission-list .picker-meta.path {
  font-family: var(--peek-font-mono, ui-monospace, monospace);
  margin: 2px 0 5px;
  min-height: 0;
  padding: 4px 12px 7px;
  border: 0;
  border-radius: 0;
  background: transparent;
  font-size: 11px;
  font-weight: 400;
  line-height: 1.5;
  color: var(--peek-text);
  word-break: break-all;
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

.path-permission-list .picker-meta.question {
  min-height: 30px;
  padding: 4px 12px;
  font-size: 12px;
  font-weight: 500;
  line-height: 1.45;
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

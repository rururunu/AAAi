<template>
  <ul
    class="command-list ask-user-list path-permission-list peek-scrollbar"
    data-tauri-drag-region="false"
    role="listbox"
    :aria-label="ariaLabel"
  >
    <li class="picker-sticky-head">
      <div class="picker-meta">
        <span class="picker-meta-label">{{ header }}</span>
      </div>
      <div class="picker-meta question">{{ question }}</div>
      <div class="picker-meta path">{{ path }}</div>
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

.ask-user-list,
.path-permission-list {
  max-height: min(
    var(--interaction-picker-max-height, 48vh),
    calc(
      var(--picker-meta-row-height) * 3 +
        var(--command-row-height) * var(--command-list-visible-rows) +
        var(--command-list-padding) +
        48px
    )
  );
}

.picker-sticky-head {
  position: sticky;
  top: 0;
  z-index: 3;
  margin: 0;
  padding: 0 0 4px;
  background: var(--peek-list-bg);
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
  background: inherit;
}

.ask-user-list .picker-meta.question {
  min-height: 28px;
  align-items: flex-start;
  padding-top: 2px;
  padding-bottom: 4px;
  line-height: 1.45;
  color: var(--peek-text);
  white-space: normal;
  overflow-wrap: anywhere;
}

.path-permission-list .picker-meta.path {
  min-height: 24px;
  padding-bottom: 6px;
  font-family: var(--font-mono);
  font-size: 11px;
  color: var(--peek-accent);
  white-space: normal;
  overflow-wrap: anywhere;
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

.path-permission-list .command-item {
  height: auto;
  min-height: 36px;
  padding: 8px 12px;
}

.permission-option-label {
  flex: 1;
  min-width: 0;
  font-size: 13px;
  color: var(--peek-text);
}

.command-item.active {
  background: var(--peek-list-active);
}

.path-permission-list .command-item.active {
  background: color-mix(in srgb, var(--peek-accent) 10%, transparent);
}
</style>

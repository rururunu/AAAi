<template>
  <ul
    class="command-list peek-scrollbar"
    :class="{ 'command-list--workbench': appearance === 'workbench' }"
    data-tauri-drag-region="false"
    role="listbox"
    :aria-label="ariaLabel"
  >
    <li
      v-for="(item, index) in commands"
      :key="item.command"
      class="command-item"
      :class="{ active: index === selectedIndex }"
      role="option"
      :aria-selected="index === selectedIndex"
      @mouseenter="$emit('hover', index)"
      @mousedown.prevent="$emit('select', item.command)"
    >
      <span v-if="item.icon" class="command-leading" aria-hidden="true">
        <component :is="item.icon" :size="14" class="command-icon" />
      </span>
      <span class="command-text">
        <span class="command-name">{{ item.command }}</span>
        <span class="command-desc">{{ item.description }}</span>
      </span>
      <kbd v-if="appearance === 'workbench'" class="command-kbd">Enter</kbd>
    </li>
  </ul>
</template>

<script setup lang="ts">
import type { Component } from "vue";

defineProps<{
  commands: Array<{
    command: string;
    description: string;
    icon?: Component;
  }>;
  selectedIndex: number;
  ariaLabel: string;
  appearance?: "overlay" | "workbench";
}>();

defineEmits<{
  hover: [index: number];
  select: [command: string];
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
    calc(
      var(--command-row-height) * var(--command-list-visible-rows) + var(--command-list-padding)
    ),
    72vh
  );
  overflow-x: hidden;
  overflow-y: auto;
  overscroll-behavior: contain;
  font-family: var(--peek-font-sans);
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

.command-leading {
  flex: none;
  width: 22px;
  height: 22px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 6px;
  background: color-mix(in srgb, var(--peek-text) 6%, transparent);
  color: var(--peek-muted);
}

.command-item.active .command-leading {
  background: color-mix(in srgb, var(--peek-accent) 16%, transparent);
  color: var(--peek-accent);
}

.command-icon {
  opacity: 0.92;
}

.command-text {
  display: flex;
  flex: 1;
  align-items: baseline;
  gap: 10px;
  min-width: 0;
}

.command-name {
  flex: none;
  font-size: 13px;
  color: var(--peek-accent);
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

.command-kbd {
  flex: none;
  margin: 0;
  padding: 1px 6px;
  border: 1px solid color-mix(in srgb, var(--peek-text) 14%, transparent);
  border-radius: 5px;
  background: color-mix(in srgb, var(--peek-text) 5%, transparent);
  color: var(--peek-muted);
  font-family: var(--peek-font-sans);
  font-size: 10px;
  font-weight: 500;
  line-height: 14px;
  letter-spacing: 0.02em;
}

/* Workbench: compact single-command card above the composer */
.command-list--workbench {
  --command-row-height: 44px;
  --command-list-visible-rows: 1;
  padding: 6px;
  max-height: none;
  overflow: hidden;
}

.command-list--workbench .command-item {
  gap: 10px;
  height: auto;
  min-height: var(--command-row-height);
  padding: 8px 10px;
  border-radius: 10px;
}

.command-list--workbench .command-item.active {
  background: color-mix(in srgb, var(--peek-accent) 10%, transparent);
}

.command-list--workbench .command-text {
  flex-direction: column;
  align-items: flex-start;
  gap: 2px;
}

.command-list--workbench .command-name {
  font-size: 13px;
  font-weight: 600;
  line-height: 16px;
  color: var(--peek-text);
}

.command-list--workbench .command-desc {
  font-size: 11px;
  line-height: 14px;
  white-space: normal;
}
</style>

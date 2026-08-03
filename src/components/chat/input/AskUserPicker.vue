<template>
  <ul
    class="command-list ask-user-list peek-scrollbar"
    data-tauri-drag-region="false"
    role="listbox"
    :aria-label="ariaLabel"
  >
    <li class="picker-sticky-head" aria-hidden="false">
      <div class="picker-meta">
        <span class="picker-meta-label">{{ header }}</span>
        <span v-if="questionCount > 1" class="picker-meta-progress">
          {{ questionIndex + 1 }}/{{ questionCount }}
        </span>
      </div>
      <div class="picker-meta question">{{ question }}</div>
    </li>
    <li
      v-for="(option, index) in options"
      :key="option.slug"
      class="command-item"
      :class="{
        active: index === selectedIndex,
        selected: isOptionSelected(option.label),
      }"
      role="option"
      :aria-selected="index === selectedIndex"
      @mouseenter="$emit('hover', index)"
      @mousedown.prevent="$emit('select', option)"
    >
      <span class="command-name">/{{ option.slug }}</span>
      <span class="command-desc">{{ option.description || option.label }}</span>
    </li>
    <li
      v-if="multiSelect"
      class="command-item confirm-item"
      :class="{ active: selectedIndex === confirmRowIndex }"
      role="option"
      @mouseenter="$emit('hover', confirmRowIndex)"
      @mousedown.prevent="$emit('confirm')"
    >
      <span class="command-name">/confirm</span>
      <span class="command-desc">{{ confirmLabel }}</span>
    </li>
  </ul>
</template>

<script setup lang="ts">
import type { AskDisplayOption } from "@/types/chat";

defineProps<{
  header?: string;
  question?: string;
  questionIndex: number;
  questionCount: number;
  options: AskDisplayOption[];
  multiSelect?: boolean;
  confirmRowIndex: number;
  confirmLabel: string;
  selectedIndex: number;
  ariaLabel: string;
  isOptionSelected: (label: string) => boolean;
}>();

defineEmits<{
  hover: [index: number];
  select: [option: AskDisplayOption];
  confirm: [];
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
    var(--interaction-picker-max-height, 48vh),
    calc(
      var(--picker-meta-row-height) * 2 +
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
  list-style: none;
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
  padding-bottom: 6px;
  line-height: 1.45;
  color: var(--peek-text);
  white-space: normal;
  overflow-wrap: anywhere;
}

.picker-meta-label {
  font-weight: 600;
  color: var(--peek-accent);
}

.picker-meta-progress {
  flex: none;
  font-variant-numeric: tabular-nums;
}

.command-item.selected {
  background: color-mix(in srgb, var(--peek-accent) 10%, transparent);
}

.command-item.confirm-item .command-name {
  color: color-mix(in srgb, var(--peek-accent) 85%, white);
}

.ask-user-list .command-item:has(.command-name) .command-name {
  font-family: var(--font-mono);
}

.ask-user-list .command-item .command-name {
  color: var(--peek-accent);
}

.ask-user-list .command-item:last-of-type:not(.confirm-item) .command-desc {
  color: var(--peek-muted);
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

.command-name {
  flex: none;
  font-family: var(--font-mono);
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
</style>
